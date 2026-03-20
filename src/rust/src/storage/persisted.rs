use std::env;
use std::fs;
use std::io;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use chrono::Utc;
use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};
use ring::pbkdf2;
use ring::rand::{SecureRandom, SystemRandom};
use rusqlite::{params, Connection, OpenFlags, OptionalExtension};
use serde::{de::DeserializeOwned, Serialize};

use crate::models::{Contact, ContactGroup, DraftMessage, StoredAccountState, SyncStatus};
use crate::protocol::{RecoveryExportStatus, StorageSecurityStatus};

const APP_DIR_NAME: &str = "MailYou";
const STORAGE_DIR_NAME: &str = "mail";
const DATABASE_FILE: &str = "storage.sqlite3";
const BACKUP_DIR_NAME: &str = "backups";
const MAX_DATABASE_BACKUPS: usize = 5;
const EXPORT_DIR_NAME: &str = "exports";
const MAX_RECOVERY_EXPORTS: usize = 10;
const RECOVERY_DIR_NAME: &str = "recovery";
const RECOVERY_KEY_FILE: &str = "storage-key.b64";
const RESET_ARCHIVE_DIR_NAME: &str = "resets";
const LEGACY_ACCOUNTS_FILE: &str = "accounts.json";
const LEGACY_DRAFTS_FILE: &str = "drafts.json";
const LEGACY_MAILBOX_FILE: &str = "mailbox.json";
const LEGACY_SYNC_FILE: &str = "sync.json";
const LEGACY_CONTACTS_FILE: &str = "contacts.json";

const KEYRING_SERVICE: &str = "mailyou";
const SECRET_PLACEHOLDER: &str = "<keyring>";
const STORAGE_KEY_ACCOUNT: &str = "__storage__";
const STORAGE_KEY_KIND: &str = "masterKey";
const STORAGE_PROBE_ACCOUNT: &str = "__storage_probe__";
const STORAGE_PROBE_KIND: &str = "healthcheck";
const KDF_ITERATIONS: u32 = 120_000;
const PASSWORD_SALT_LEN: usize = 16;

const META_SECURITY_MODE: &str = "securityMode";
const META_WRAPPED_KEY_SALT: &str = "wrappedKeySalt";
const META_WRAPPED_KEY_NONCE: &str = "wrappedKeyNonce";
const META_WRAPPED_KEY_CIPHERTEXT: &str = "wrappedKeyCiphertext";
const META_STORAGE_SCHEMA_VERSION: &str = "storageSchemaVersion";
const CURRENT_STORAGE_SCHEMA_VERSION: u32 = 1;

const MODE_KEYRING: &str = "keyring";
const MODE_PASSWORD: &str = "password";

const STATE_ACCOUNTS: &str = "accounts";
const STATE_DRAFTS: &str = "drafts";
const STATE_MAILBOX: &str = "mailbox";
const STATE_SYNC: &str = "sync";
const STATE_CONTACTS: &str = "contacts";
const STATE_FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone)]
struct EncryptedPayload {
    nonce: Vec<u8>,
    ciphertext: Vec<u8>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct PersistedStateEnvelope<T> {
    version: u32,
    payload: T,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct RecoveryExport {
    #[serde(default = "default_recovery_export_version")]
    version: u32,
    #[serde(default)]
    exported_at: String,
    #[serde(default)]
    accounts: Vec<StoredAccountState>,
    #[serde(default)]
    drafts: Vec<DraftMessage>,
    #[serde(default)]
    contacts: PersistedContacts,
}

fn default_recovery_export_version() -> u32 {
    1
}

fn unlocked_storage_key() -> &'static Mutex<Option<[u8; 32]>> {
    static STORAGE_KEY: OnceLock<Mutex<Option<[u8; 32]>>> = OnceLock::new();
    STORAGE_KEY.get_or_init(|| Mutex::new(None))
}

fn degraded_storage_reads() -> &'static AtomicBool {
    static FLAG: OnceLock<AtomicBool> = OnceLock::new();
    FLAG.get_or_init(|| AtomicBool::new(false))
}

fn degraded_storage_read_error() -> &'static Mutex<Option<String>> {
    static ERROR: OnceLock<Mutex<Option<String>>> = OnceLock::new();
    ERROR.get_or_init(|| Mutex::new(None))
}

fn session_backup_created() -> &'static AtomicBool {
    static FLAG: OnceLock<AtomicBool> = OnceLock::new();
    FLAG.get_or_init(|| AtomicBool::new(false))
}

fn mark_degraded_storage_read() {
    degraded_storage_reads().store(true, Ordering::Relaxed);
}

fn record_degraded_storage_read_error(error: &io::Error) {
    if is_missing_persisted_state_error(error) {
        return;
    }
    mark_degraded_storage_read();
    let mut slot = degraded_storage_read_error().lock().unwrap();
    if slot.is_none() {
        *slot = Some(error.to_string());
    }
}

fn is_missing_persisted_state_error(error: &io::Error) -> bool {
    error.kind() == io::ErrorKind::NotFound
        && error
            .to_string()
            .starts_with("Missing persisted state:")
}

pub fn storage_reads_degraded() -> bool {
    degraded_storage_reads().load(Ordering::Relaxed)
}

pub fn degraded_storage_error_message() -> Option<String> {
    degraded_storage_read_error().lock().unwrap().clone()
}

pub fn load_accounts() -> Vec<StoredAccountState> {
    let mut accounts: Vec<StoredAccountState> = match load_state_json(STATE_ACCOUNTS) {
        Ok(accounts) => accounts,
        Err(error) if is_missing_persisted_state_error(&error) => Vec::new(),
        Err(error) if error.kind() == io::ErrorKind::PermissionDenied => {
            panic!("storage locked: {}", error)
        }
        Err(error) => {
            eprintln!("[store] WARNING: failed to load accounts state: {}", error);
            record_degraded_storage_read_error(&error);
            Vec::new()
        }
    };
    hydrate_accounts_with_keyring_secrets(&mut accounts, false);

    accounts
}

pub fn load_drafts() -> Vec<DraftMessage> {
    match load_state_json(STATE_DRAFTS) {
        Ok(drafts) => drafts,
        Err(error) if is_missing_persisted_state_error(&error) => Vec::new(),
        Err(error) if error.kind() == io::ErrorKind::PermissionDenied => {
            panic!("storage locked: {}", error)
        }
        Err(error) => {
            eprintln!("[store] WARNING: failed to load drafts state: {}", error);
            record_degraded_storage_read_error(&error);
            Vec::new()
        }
    }
}

pub fn load_mailbox() -> PersistedMailbox {
    match load_state_json(STATE_MAILBOX) {
        Ok(mailbox) => mailbox,
        Err(error) if is_missing_persisted_state_error(&error) => PersistedMailbox::default(),
        Err(error) if error.kind() == io::ErrorKind::PermissionDenied => {
            panic!("storage locked: {}", error)
        }
        Err(error) => {
            eprintln!("[store] WARNING: failed to load mailbox state: {}", error);
            record_degraded_storage_read_error(&error);
            PersistedMailbox::default()
        }
    }
}

pub fn load_sync_statuses() -> Vec<SyncStatus> {
    match load_state_json(STATE_SYNC) {
        Ok(statuses) => statuses,
        Err(error) if is_missing_persisted_state_error(&error) => Vec::new(),
        Err(error) if error.kind() == io::ErrorKind::PermissionDenied => {
            panic!("storage locked: {}", error)
        }
        Err(error) => {
            eprintln!("[store] WARNING: failed to load sync state: {}", error);
            record_degraded_storage_read_error(&error);
            Vec::new()
        }
    }
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistedContacts {
    pub contacts: Vec<Contact>,
    pub groups: Vec<ContactGroup>,
}

pub fn load_contacts() -> PersistedContacts {
    let mut data: PersistedContacts = match load_state_json(STATE_CONTACTS) {
        Ok(data) => data,
        Err(error) if is_missing_persisted_state_error(&error) => PersistedContacts::default(),
        Err(error) if error.kind() == io::ErrorKind::PermissionDenied => {
            panic!("storage locked: {}", error)
        }
        Err(error) => {
            eprintln!("[store] WARNING: failed to load contacts state: {}", error);
            record_degraded_storage_read_error(&error);
            PersistedContacts::default()
        }
    };
    hydrate_contact_avatar_tokens(&mut data.contacts);
    data
}

pub fn save_snapshot(
    accounts: &[StoredAccountState],
    drafts: &[DraftMessage],
    mailbox: &PersistedMailbox,
    sync_statuses: &[SyncStatus],
    contacts: &PersistedContacts,
) -> io::Result<()> {
    ensure_initialized()?;
    let mut connection = open_connection()?;
    let transaction = connection.transaction().map_err(to_io_error)?;
    let sanitized_accounts = sanitize_accounts_for_persistence(accounts);

    write_state_json_with_connection(&transaction, STATE_ACCOUNTS, &sanitized_accounts)?;
    write_state_json_with_connection(&transaction, STATE_DRAFTS, drafts)?;
    write_state_json_with_connection(&transaction, STATE_MAILBOX, mailbox)?;
    write_state_json_with_connection(&transaction, STATE_SYNC, sync_statuses)?;
    write_state_json_with_connection(&transaction, STATE_CONTACTS, contacts)?;

    transaction.commit().map_err(to_io_error)?;
    write_recovery_export(&sanitized_accounts, drafts, contacts)?;
    Ok(())
}

pub fn has_contacts_file() -> bool {
    has_state(STATE_CONTACTS)
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistedMailbox {
    pub folders: Vec<crate::models::MailboxFolder>,
    pub messages: Vec<crate::models::MailMessage>,
    pub threads: Vec<crate::models::MailThread>,
}

pub fn has_accounts_file() -> bool {
    has_state(STATE_ACCOUNTS)
}

pub fn has_mailbox_file() -> bool {
    has_state(STATE_MAILBOX)
}

pub fn has_drafts_file() -> bool {
    has_state(STATE_DRAFTS)
}

pub fn storage_dir_path() -> io::Result<PathBuf> {
    storage_dir()
}

pub fn get_recovery_export_status() -> io::Result<RecoveryExportStatus> {
    let export_dir = export_dir()?;
    fs::create_dir_all(&export_dir)?;
    let latest_path = export_dir.join("latest.json");
    let latest_exported_at = if latest_path.exists() {
        fs::metadata(&latest_path)
            .and_then(|meta| meta.modified())
            .ok()
            .map(|time| chrono::DateTime::<Utc>::from(time).to_rfc3339())
    } else {
        None
    };

    let snapshot_count = fs::read_dir(&export_dir)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_name().to_string_lossy().starts_with("recovery-"))
        .filter(|entry| entry.path().extension().and_then(|ext| ext.to_str()) == Some("json"))
        .count() as u32;

    Ok(RecoveryExportStatus {
        export_dir: export_dir.to_string_lossy().to_string(),
        latest_exported_at,
        snapshot_count,
    })
}

pub fn restore_latest_recovery_export() -> io::Result<()> {
    let mut export = load_latest_recovery_export()?;
    hydrate_accounts_with_keyring_secrets(&mut export.accounts, true);

    archive_storage_dir("mail-recovery")?;

    let storage_path = storage_dir()?;
    fs::create_dir_all(&storage_path)?;
    keyring_delete(STORAGE_KEY_ACCOUNT, STORAGE_KEY_KIND);
    *unlocked_storage_key().lock().unwrap() = None;
    degraded_storage_reads().store(false, Ordering::Relaxed);
    *degraded_storage_read_error().lock().unwrap() = None;
    session_backup_created().store(false, Ordering::Relaxed);

    let mailbox = PersistedMailbox::default();
    let sync_statuses: Vec<SyncStatus> = Vec::new();
    save_snapshot(
        &export.accounts,
        &export.drafts,
        &mailbox,
        &sync_statuses,
        &export.contacts,
    )?;

    eprintln!(
        "[store] restored latest recovery export ({} accounts, {} drafts, {} contacts)",
        export.accounts.len(),
        export.drafts.len(),
        export.contacts.contacts.len()
    );
    Ok(())
}

pub fn reset_local_encrypted_storage() -> io::Result<()> {
    let storage_path = storage_dir()?;
    archive_storage_dir("mail-reset")?;

    fs::create_dir_all(&storage_path)?;
    keyring_delete(STORAGE_KEY_ACCOUNT, STORAGE_KEY_KIND);
    *unlocked_storage_key().lock().unwrap() = None;
    degraded_storage_reads().store(false, Ordering::Relaxed);
    *degraded_storage_read_error().lock().unwrap() = None;
    session_backup_created().store(false, Ordering::Relaxed);
    Ok(())
}

pub fn save_raw_email(message_id: &str, raw: &[u8]) -> io::Result<()> {
    save_binary("raw_email", message_id, "message/rfc822", raw)
}

pub fn load_raw_email(message_id: &str) -> io::Result<Vec<u8>> {
    if let Some(raw) = load_binary("raw_email", message_id)? {
        return Ok(raw);
    }

    let path = legacy_raw_path(message_id)?;
    let raw = fs::read(&path)?;
    let _ = save_raw_email(message_id, &raw);
    Ok(raw)
}

pub fn save_avatar(contact_id: &str, mime_type: &str, payload: &[u8]) -> io::Result<()> {
    save_binary("avatar", contact_id, mime_type, payload)
}

pub fn load_avatar(contact_id: &str) -> io::Result<Option<(String, Vec<u8>)>> {
    if let Some(entry) = load_binary_with_mime("avatar", contact_id)? {
        return Ok(Some(entry));
    }

    if let Some((mime_type, payload)) = load_legacy_avatar(contact_id)? {
        let _ = save_avatar(contact_id, &mime_type, &payload);
        return Ok(Some((mime_type, payload)));
    }

    Ok(None)
}

pub fn delete_avatar(contact_id: &str) -> io::Result<()> {
    delete_binary("avatar", contact_id)?;
    if let Some(path) = legacy_avatar_path(contact_id)? {
        let _ = fs::remove_file(path);
    }
    Ok(())
}

pub fn get_security_status() -> io::Result<StorageSecurityStatus> {
    ensure_initialized()?;
    let connection = open_connection()?;
    let mode = current_security_mode(&connection)?;
    let (keyring_available, probe_error) = probe_system_keyring();
    let has_recovery_key_backup = recovery_key_path()?.exists();
    let has_encrypted_data = persisted_storage_requires_existing_key().unwrap_or(false);
    let startup_read_error = degraded_storage_error_message();
    let storage_key_error = if matches!(mode, MODE_KEYRING) {
        data_key_from_keyring()
            .and_then(|storage_key| probe_existing_state_decryption(&connection, &storage_key))
            .err()
            .map(|error| error.to_string())
    } else {
        None
    };
    let keyring_error = startup_read_error.or(storage_key_error).or(probe_error);
    let is_unlocked = match mode {
        MODE_KEYRING => keyring_error.is_none(),
        MODE_PASSWORD => unlocked_storage_key().lock().unwrap().is_some(),
        _ => false,
    };

    Ok(StorageSecurityStatus {
        has_master_password: matches!(mode, MODE_PASSWORD),
        is_unlocked,
        mode,
        keyring_available,
        keyring_error,
        has_recovery_key_backup,
        master_password_recommended: matches!(mode, MODE_KEYRING) && has_encrypted_data,
    })
}

pub fn unlock_storage(password: &str) -> io::Result<StorageSecurityStatus> {
    ensure_initialized()?;
    let connection = open_connection()?;
    let mode = current_security_mode(&connection)?;
    if !matches!(mode, MODE_PASSWORD) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Master password is not configured",
        ));
    }

    let data_key = decrypt_wrapped_data_key(&connection, password)?;
    *unlocked_storage_key().lock().unwrap() = Some(data_key);
    get_security_status()
}

pub fn set_master_password(
    current_password: Option<&str>,
    new_password: &str,
) -> io::Result<StorageSecurityStatus> {
    ensure_initialized()?;
    let connection = open_connection()?;
    let mode = current_security_mode(&connection)?;
    let data_key = if matches!(mode, MODE_PASSWORD) {
        let password = current_password.ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "Current master password is required",
            )
        })?;
        decrypt_wrapped_data_key(&connection, password)?
    } else {
        data_key_from_keyring()?
    };

    persist_wrapped_data_key(&connection, new_password, &data_key)?;
    set_metadata(&connection, META_SECURITY_MODE, MODE_PASSWORD)?;
    keyring_delete(STORAGE_KEY_ACCOUNT, STORAGE_KEY_KIND);
    *unlocked_storage_key().lock().unwrap() = Some(data_key);
    get_security_status()
}

pub fn clear_master_password(current_password: &str) -> io::Result<StorageSecurityStatus> {
    ensure_initialized()?;
    let connection = open_connection()?;
    let mode = current_security_mode(&connection)?;
    if !matches!(mode, MODE_PASSWORD) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Master password is not configured",
        ));
    }

    let data_key = decrypt_wrapped_data_key(&connection, current_password)?;
    store_data_key_in_keyring(&data_key)?;
    delete_metadata(&connection, META_WRAPPED_KEY_SALT)?;
    delete_metadata(&connection, META_WRAPPED_KEY_NONCE)?;
    delete_metadata(&connection, META_WRAPPED_KEY_CIPHERTEXT)?;
    set_metadata(&connection, META_SECURITY_MODE, MODE_KEYRING)?;
    *unlocked_storage_key().lock().unwrap() = None;
    get_security_status()
}

fn load_state_json<T: DeserializeOwned>(key: &str) -> io::Result<T> {
    let Some(payload) = read_entry("state", key)? else {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Missing persisted state: {key}"),
        ));
    };
    let plaintext = decrypt_payload(&payload)?;

    if let Ok(envelope) = serde_json::from_slice::<PersistedStateEnvelope<T>>(&plaintext) {
        return Ok(envelope.payload);
    }

    match serde_json::from_slice::<T>(&plaintext) {
        Ok(value) => Ok(value),
        Err(error) => {
            eprintln!(
                "[store] WARNING: failed to deserialize persisted state key={} error={}",
                key, error
            );
            Err(io::Error::new(io::ErrorKind::InvalidData, error.to_string()))
        }
    }
}

fn write_state_json_with_connection<T: Serialize + ?Sized>(
    connection: &Connection,
    key: &str,
    value: &T,
) -> io::Result<()> {
    let plaintext = serde_json::to_vec(&PersistedStateEnvelope {
        version: STATE_FORMAT_VERSION,
        payload: value,
    })
    .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))?;
    let payload = encrypt_payload(&plaintext)?;
    write_entry_with_connection(connection, "state", key, None, &payload)
}

fn has_state(key: &str) -> bool {
    match ensure_initialized_for_read().and_then(|_| read_entry("state", key).map(|entry| entry.is_some())) {
        Ok(result) => result,
        Err(error) => {
            record_degraded_storage_read_error(&error);
            eprintln!(
                "[store] WARNING: failed to probe persisted state key={} error={}",
                key, error
            );
            false
        }
    }
}

fn save_binary(kind: &str, key: &str, mime_type: &str, payload: &[u8]) -> io::Result<()> {
    let encrypted = encrypt_payload(payload)?;
    write_entry(kind, key, Some(mime_type), &encrypted)
}

fn load_binary(kind: &str, key: &str) -> io::Result<Option<Vec<u8>>> {
    Ok(load_binary_with_mime(kind, key)?.map(|(_, payload)| payload))
}

fn load_binary_with_mime(kind: &str, key: &str) -> io::Result<Option<(String, Vec<u8>)>> {
    let Some((mime_type, payload)) = read_entry_with_mime(kind, key)? else {
        return Ok(None);
    };
    let plaintext = decrypt_payload(&payload)?;
    Ok(Some((
        mime_type.unwrap_or_else(|| "application/octet-stream".into()),
        plaintext,
    )))
}

fn delete_binary(kind: &str, key: &str) -> io::Result<()> {
    ensure_initialized()?;
    let connection = open_connection()?;
    connection
        .execute(
            "DELETE FROM encrypted_entries WHERE kind = ?1 AND entry_key = ?2",
            params![kind, key],
        )
        .map_err(to_io_error)?;
    Ok(())
}

fn write_entry(
    kind: &str,
    key: &str,
    mime_type: Option<&str>,
    payload: &EncryptedPayload,
) -> io::Result<()> {
    ensure_initialized()?;
    let connection = open_connection()?;
    write_entry_with_connection(&connection, kind, key, mime_type, payload)
}

fn write_entry_with_connection(
    connection: &Connection,
    kind: &str,
    key: &str,
    mime_type: Option<&str>,
    payload: &EncryptedPayload,
) -> io::Result<()> {
    connection
        .execute(
            "INSERT INTO encrypted_entries (kind, entry_key, mime_type, nonce, ciphertext)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(kind, entry_key) DO UPDATE SET
               mime_type = excluded.mime_type,
               nonce = excluded.nonce,
               ciphertext = excluded.ciphertext",
            params![kind, key, mime_type, payload.nonce, payload.ciphertext],
        )
        .map_err(to_io_error)?;
    Ok(())
}

fn read_entry(kind: &str, key: &str) -> io::Result<Option<EncryptedPayload>> {
    Ok(read_entry_with_mime(kind, key)?.map(|(_, payload)| payload))
}

fn read_entry_with_mime(
    kind: &str,
    key: &str,
) -> io::Result<Option<(Option<String>, EncryptedPayload)>> {
    ensure_initialized_for_read()?;
    let connection = open_connection_for_read()?;
    connection
        .query_row(
            "SELECT mime_type, nonce, ciphertext
             FROM encrypted_entries
             WHERE kind = ?1 AND entry_key = ?2",
            params![kind, key],
            |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    EncryptedPayload {
                        nonce: row.get(1)?,
                        ciphertext: row.get(2)?,
                    },
                ))
            },
        )
        .optional()
        .map_err(to_io_error)
}

fn ensure_initialized() -> io::Result<()> {
    let storage_dir = storage_dir()?;
    fs::create_dir_all(&storage_dir)?;
    let database_exists = database_path()?.exists();
    let connection = open_connection()?;
    initialize_schema(&connection)?;
    if database_exists {
        ensure_database_backup(&connection)?;
    }
    migrate_legacy_state_if_needed(&connection)?;
    migrate_storage_schema_if_needed(&connection)?;
    Ok(())
}

fn ensure_initialized_for_read() -> io::Result<()> {
    let storage_dir = storage_dir()?;
    fs::create_dir_all(&storage_dir)?;

    match open_connection() {
        Ok(connection) => {
            initialize_schema(&connection)?;
            migrate_legacy_state_if_needed(&connection)?;
            Ok(())
        }
        Err(error) => {
            if database_path()?.exists() {
                record_degraded_storage_read_error(&error);
                eprintln!(
                    "[store] WARNING: falling back to read-only persisted storage access: {}",
                    error
                );
                Ok(())
            } else {
                Err(error)
            }
        }
    }
}

fn open_connection() -> io::Result<Connection> {
    let db_path = database_path()?;
    let connection = Connection::open(db_path).map_err(to_io_error)?;
    connection
        .execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA foreign_keys = ON;",
        )
        .map_err(to_io_error)?;
    Ok(connection)
}

fn open_connection_for_read() -> io::Result<Connection> {
    match open_connection() {
        Ok(connection) => Ok(connection),
        Err(error) => {
            let db_path = database_path()?;
            if !db_path.exists() {
                return Err(error);
            }

            let uri = format!("file:{}?mode=ro&immutable=1", db_path.to_string_lossy());
            if let Ok(connection) = Connection::open_with_flags(
                uri,
                OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI,
            ) {
                return Ok(connection);
            }

            let temp_db_path = env::temp_dir().join(format!(
                "mailyou-storage-readonly-{}.sqlite3",
                std::process::id()
            ));
            fs::copy(&db_path, &temp_db_path)?;
            Connection::open_with_flags(temp_db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
                .map_err(to_io_error)
        }
    }
}

fn initialize_schema(connection: &Connection) -> io::Result<()> {
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS encrypted_entries (
               kind TEXT NOT NULL,
               entry_key TEXT NOT NULL,
               mime_type TEXT,
               nonce BLOB NOT NULL,
               ciphertext BLOB NOT NULL,
               PRIMARY KEY (kind, entry_key)
             );
             CREATE TABLE IF NOT EXISTS metadata (
               key TEXT PRIMARY KEY,
               value TEXT NOT NULL
             );",
        )
        .map_err(to_io_error)?;
    Ok(())
}

fn current_security_mode(connection: &Connection) -> io::Result<&'static str> {
    match get_metadata(connection, META_SECURITY_MODE)?.as_deref() {
        Some(MODE_PASSWORD) => Ok(MODE_PASSWORD),
        _ => Ok(MODE_KEYRING),
    }
}

fn get_metadata(connection: &Connection, key: &str) -> io::Result<Option<String>> {
    connection
        .query_row(
            "SELECT value FROM metadata WHERE key = ?1",
            params![key],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(to_io_error)
}

fn set_metadata(connection: &Connection, key: &str, value: &str) -> io::Result<()> {
    connection
        .execute(
            "INSERT INTO metadata (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )
        .map_err(to_io_error)?;
    Ok(())
}

fn delete_metadata(connection: &Connection, key: &str) -> io::Result<()> {
    connection
        .execute("DELETE FROM metadata WHERE key = ?1", params![key])
        .map_err(to_io_error)?;
    Ok(())
}

fn migrate_legacy_state_if_needed(connection: &Connection) -> io::Result<()> {
    let already_migrated = connection
        .query_row(
            "SELECT value FROM metadata WHERE key = 'legacyMigrationCompleted'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(to_io_error)?
        .as_deref()
        == Some("1");

    if already_migrated {
        return Ok(());
    }

    let state_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM encrypted_entries WHERE kind = 'state'",
            [],
            |row| row.get(0),
        )
        .map_err(to_io_error)?;

    if state_count == 0 {
        migrate_legacy_json_files(connection)?;
        migrate_legacy_raw_emails(connection)?;
        migrate_legacy_avatars(connection)?;
    }

    connection
        .execute(
            "INSERT INTO metadata (key, value) VALUES ('legacyMigrationCompleted', '1')
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [],
        )
        .map_err(to_io_error)?;

    Ok(())
}

fn migrate_storage_schema_if_needed(connection: &Connection) -> io::Result<()> {
    let stored_version = get_metadata(connection, META_STORAGE_SCHEMA_VERSION)?
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or_else(|| infer_storage_schema_version(connection).unwrap_or(0));

    if stored_version >= CURRENT_STORAGE_SCHEMA_VERSION {
        return Ok(());
    }

    let Some(storage_key) = active_storage_key(connection)? else {
        eprintln!(
            "[store] storage schema migration postponed because encrypted state is locked"
        );
        return Ok(());
    };

    if stored_version < 1 {
        normalize_state_envelopes(connection, &storage_key)?;
    }

    set_metadata(
        connection,
        META_STORAGE_SCHEMA_VERSION,
        &CURRENT_STORAGE_SCHEMA_VERSION.to_string(),
    )?;
    eprintln!(
        "[store] migrated persisted storage schema from v{} to v{}",
        stored_version, CURRENT_STORAGE_SCHEMA_VERSION
    );
    Ok(())
}

fn migrate_legacy_json_files(connection: &Connection) -> io::Result<()> {
    migrate_legacy_state_file::<Vec<StoredAccountState>, _>(
        connection,
        LEGACY_ACCOUNTS_FILE,
        STATE_ACCOUNTS,
        legacy_load_accounts,
    )?;
    migrate_legacy_state_file::<Vec<DraftMessage>, _>(
        connection,
        LEGACY_DRAFTS_FILE,
        STATE_DRAFTS,
        legacy_load_json,
    )?;
    migrate_legacy_state_file::<PersistedMailbox, _>(
        connection,
        LEGACY_MAILBOX_FILE,
        STATE_MAILBOX,
        legacy_load_json,
    )?;
    migrate_legacy_state_file::<Vec<SyncStatus>, _>(
        connection,
        LEGACY_SYNC_FILE,
        STATE_SYNC,
        legacy_load_json,
    )?;
    migrate_legacy_state_file::<PersistedContacts, _>(
        connection,
        LEGACY_CONTACTS_FILE,
        STATE_CONTACTS,
        |path| {
            let mut contacts: PersistedContacts = legacy_load_json(path)?;
            hydrate_contact_avatar_tokens(&mut contacts.contacts);
            Ok(contacts)
        },
    )?;
    Ok(())
}

fn migrate_legacy_state_file<T, F>(
    connection: &Connection,
    file_name: &str,
    state_key: &str,
    loader: F,
) -> io::Result<()>
where
    T: Serialize,
    F: Fn(&Path) -> io::Result<T>,
{
    let path = storage_dir()?.join(file_name);
    if !path.exists() {
        return Ok(());
    }

    let value = loader(&path)?;
    let plaintext = serde_json::to_vec(&value)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))?;
    let encrypted = encrypt_payload(&plaintext)?;
    connection
        .execute(
            "INSERT OR REPLACE INTO encrypted_entries (kind, entry_key, mime_type, nonce, ciphertext)
             VALUES ('state', ?1, NULL, ?2, ?3)",
            params![state_key, encrypted.nonce, encrypted.ciphertext],
        )
        .map_err(to_io_error)?;
    Ok(())
}

fn infer_storage_schema_version(connection: &Connection) -> io::Result<u32> {
    let state_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM encrypted_entries WHERE kind = 'state'",
            [],
            |row| row.get(0),
        )
        .map_err(to_io_error)?;

    if state_count == 0 {
        Ok(CURRENT_STORAGE_SCHEMA_VERSION)
    } else {
        Ok(0)
    }
}

fn active_storage_key(connection: &Connection) -> io::Result<Option<[u8; 32]>> {
    match current_security_mode(connection)? {
        MODE_KEYRING => data_key_from_keyring().map(Some),
        MODE_PASSWORD => Ok(unlocked_storage_key().lock().unwrap().as_ref().copied()),
        _ => Err(io::Error::other("Unknown storage security mode")),
    }
}

fn sanitize_accounts_for_persistence(accounts: &[StoredAccountState]) -> Vec<StoredAccountState> {
    let mut sanitized = accounts.to_vec();

    for account_state in &mut sanitized {
        let account_id = &account_state.account.id;
        let password = &account_state.config.password;

        if !password.is_empty()
            && password != SECRET_PLACEHOLDER
            && keyring_set(account_id, "password", password)
        {
            account_state.config.password = SECRET_PLACEHOLDER.into();
        }

        let refresh_token = &account_state.config.refresh_token;
        if !refresh_token.is_empty()
            && refresh_token != SECRET_PLACEHOLDER
            && keyring_set(account_id, "refreshToken", refresh_token)
        {
            account_state.config.refresh_token = SECRET_PLACEHOLDER.into();
        }
    }

    sanitized
}

fn hydrate_accounts_with_keyring_secrets(
    accounts: &mut [StoredAccountState],
    clear_unresolved_placeholders: bool,
) {
    for account_state in accounts {
        if account_state.config.password == SECRET_PLACEHOLDER
            || account_state.config.password.is_empty()
        {
            if let Some(password) = keyring_get(&account_state.account.id, "password") {
                account_state.config.password = password;
            } else if clear_unresolved_placeholders
                && account_state.config.password == SECRET_PLACEHOLDER
            {
                account_state.config.password.clear();
            }
        }

        if account_state.config.refresh_token == SECRET_PLACEHOLDER
            || account_state.config.refresh_token.is_empty()
        {
            if let Some(refresh_token) = keyring_get(&account_state.account.id, "refreshToken") {
                account_state.config.refresh_token = refresh_token;
            } else if clear_unresolved_placeholders
                && account_state.config.refresh_token == SECRET_PLACEHOLDER
            {
                account_state.config.refresh_token.clear();
            }
        }
    }
}

fn normalize_state_envelopes(connection: &Connection, storage_key: &[u8; 32]) -> io::Result<()> {
    let mut statement = connection
        .prepare(
            "SELECT entry_key, nonce, ciphertext
             FROM encrypted_entries
             WHERE kind = 'state'",
        )
        .map_err(to_io_error)?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                EncryptedPayload {
                    nonce: row.get(1)?,
                    ciphertext: row.get(2)?,
                },
            ))
        })
        .map_err(to_io_error)?;

    for row in rows {
        let (entry_key, encrypted_payload) = row.map_err(to_io_error)?;
        let plaintext = decrypt_with_key(storage_key, &encrypted_payload)?;
        if serde_json::from_slice::<PersistedStateEnvelope<serde_json::Value>>(&plaintext).is_ok() {
            continue;
        }

        let legacy_payload: serde_json::Value = serde_json::from_slice(&plaintext).map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed to decode legacy persisted state key={entry_key}: {error}"),
            )
        })?;
        let migrated_plaintext = serde_json::to_vec(&PersistedStateEnvelope {
            version: STATE_FORMAT_VERSION,
            payload: legacy_payload,
        })
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))?;
        let migrated_payload = encrypt_with_key(storage_key, &migrated_plaintext)?;
        connection
            .execute(
                "UPDATE encrypted_entries
                 SET nonce = ?1, ciphertext = ?2
                 WHERE kind = 'state' AND entry_key = ?3",
                params![migrated_payload.nonce, migrated_payload.ciphertext, entry_key],
            )
            .map_err(to_io_error)?;
    }

    Ok(())
}

fn migrate_legacy_raw_emails(_connection: &Connection) -> io::Result<()> {
    // Raw .eml files were historically stored with sanitized file names, so the
    // original message ids cannot be reconstructed reliably for bulk migration.
    // `load_raw_email` keeps a transparent fallback path and upgrades each file
    // lazily the first time it is requested.
    Ok(())
}

fn migrate_legacy_avatars(connection: &Connection) -> io::Result<()> {
    let path = storage_dir()?.join(LEGACY_CONTACTS_FILE);
    if !path.exists() {
        return Ok(());
    }

    let contacts: PersistedContacts = legacy_load_json(&path)?;
    for contact in contacts.contacts {
        if let Some((mime_type, payload)) = load_legacy_avatar(&contact.id)? {
            let encrypted = encrypt_payload(&payload)?;
            connection
                .execute(
                    "INSERT OR IGNORE INTO encrypted_entries (kind, entry_key, mime_type, nonce, ciphertext)
                     VALUES ('avatar', ?1, ?2, ?3, ?4)",
                    params![contact.id, mime_type, encrypted.nonce, encrypted.ciphertext],
                )
                .map_err(to_io_error)?;
        }
    }
    Ok(())
}

fn hydrate_contact_avatar_tokens(contacts: &mut [Contact]) {
    for contact in contacts {
        if contact.avatar_path.is_some() {
            contact.avatar_path = Some(contact.id.clone());
        }
    }
}

fn encrypt_payload(plaintext: &[u8]) -> io::Result<EncryptedPayload> {
    let key_material = storage_key()?;
    encrypt_with_key(&key_material, plaintext)
}

fn decrypt_payload(payload: &EncryptedPayload) -> io::Result<Vec<u8>> {
    if payload.nonce.len() != 12 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Invalid nonce length in persisted payload",
        ));
    }

    let key_material = storage_key()?;
    decrypt_with_key(&key_material, payload)
}

fn encrypt_with_key(key_material: &[u8; 32], plaintext: &[u8]) -> io::Result<EncryptedPayload> {
    let unbound = UnboundKey::new(&AES_256_GCM, key_material)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid storage key"))?;
    let key = LessSafeKey::new(unbound);

    let mut nonce = [0_u8; 12];
    SystemRandom::new()
        .fill(&mut nonce)
        .map_err(|_| io::Error::other("Failed to generate random nonce"))?;

    let mut in_out = plaintext.to_vec();
    key.seal_in_place_append_tag(
        Nonce::assume_unique_for_key(nonce),
        Aad::empty(),
        &mut in_out,
    )
    .map_err(|_| io::Error::other("Failed to encrypt payload"))?;

    Ok(EncryptedPayload {
        nonce: nonce.to_vec(),
        ciphertext: in_out,
    })
}

fn decrypt_with_key(key_material: &[u8; 32], payload: &EncryptedPayload) -> io::Result<Vec<u8>> {
    let unbound = UnboundKey::new(&AES_256_GCM, key_material)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid storage key"))?;
    let key = LessSafeKey::new(unbound);
    let mut nonce = [0_u8; 12];
    nonce.copy_from_slice(&payload.nonce);

    let mut in_out = payload.ciphertext.clone();
    let plaintext = key
        .open_in_place(
            Nonce::assume_unique_for_key(nonce),
            Aad::empty(),
            &mut in_out,
        )
        .map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "Failed to decrypt stored payload",
            )
        })?;

    Ok(plaintext.to_vec())
}

fn storage_key() -> io::Result<[u8; 32]> {
    let connection = open_connection_for_read()?;
    match current_security_mode(&connection)? {
        MODE_PASSWORD => unlocked_storage_key()
            .lock()
            .unwrap()
            .as_ref()
            .copied()
            .ok_or_else(|| io::Error::new(io::ErrorKind::PermissionDenied, "Storage is locked")),
        MODE_KEYRING => data_key_from_keyring(),
        _ => Err(io::Error::other("Unknown storage security mode")),
    }
}

fn data_key_from_keyring() -> io::Result<[u8; 32]> {
    if let Some(encoded) = keyring_get(STORAGE_KEY_ACCOUNT, STORAGE_KEY_KIND) {
        let key = decode_storage_key(&encoded)?;
        if !persisted_storage_requires_existing_key()? || validate_storage_key_against_existing_data(&key).is_ok()
        {
            let _ = persist_recovery_key_file(&key);
            return Ok(key);
        }

        if let Some(recovery_key) = load_recovery_key_file()? {
            if validate_storage_key_against_existing_data(&recovery_key).is_ok() {
                let _ = store_data_key_in_keyring(&recovery_key);
                return Ok(recovery_key);
            }
        }

        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Stored system keyring key no longer matches existing encrypted data",
        ));
    }

    if let Some(recovery_key) = load_recovery_key_file()? {
        if !persisted_storage_requires_existing_key()? || validate_storage_key_against_existing_data(&recovery_key).is_ok()
        {
            let _ = store_data_key_in_keyring(&recovery_key);
            return Ok(recovery_key);
        }
    }

    if persisted_storage_requires_existing_key()? {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "Missing storage key in system keyring; existing encrypted data cannot be decrypted",
        ));
    }

    let mut key = [0_u8; 32];
    SystemRandom::new()
        .fill(&mut key)
        .map_err(|_| io::Error::other("Failed to generate storage key"))?;
    store_data_key_in_keyring(&key)?;
    persist_recovery_key_file(&key)?;
    Ok(key)
}

fn decode_storage_key(encoded: &str) -> io::Result<[u8; 32]> {
    let raw = BASE64
        .decode(encoded)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))?;
    if raw.len() == 32 {
        let mut key = [0_u8; 32];
        key.copy_from_slice(&raw);
        return Ok(key);
    }

    Err(io::Error::new(
        io::ErrorKind::InvalidData,
        "Invalid persisted storage key",
    ))
}

fn store_data_key_in_keyring(key: &[u8; 32]) -> io::Result<()> {
    let encoded = BASE64.encode(key);
    if keyring_set(STORAGE_KEY_ACCOUNT, STORAGE_KEY_KIND, &encoded) {
        let _ = persist_recovery_key_file(key);
        Ok(())
    } else {
        Err(io::Error::other(
            "System keyring is unavailable; cannot store encrypted data key",
        ))
    }
}

fn decrypt_wrapped_data_key(connection: &Connection, password: &str) -> io::Result<[u8; 32]> {
    let salt = decode_metadata_base64(connection, META_WRAPPED_KEY_SALT)?;
    let nonce = decode_metadata_base64(connection, META_WRAPPED_KEY_NONCE)?;
    let ciphertext = decode_metadata_base64(connection, META_WRAPPED_KEY_CIPHERTEXT)?;
    let wrapping_key = derive_password_key(password, &salt)?;
    let payload = EncryptedPayload { nonce, ciphertext };
    let plaintext = decrypt_with_key(&wrapping_key, &payload)
        .map_err(|_| io::Error::new(io::ErrorKind::PermissionDenied, "Invalid master password"))?;
    if plaintext.len() != 32 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Wrapped data key has invalid length",
        ));
    }
    let mut key = [0_u8; 32];
    key.copy_from_slice(&plaintext);
    Ok(key)
}

fn persist_wrapped_data_key(
    connection: &Connection,
    password: &str,
    data_key: &[u8; 32],
) -> io::Result<()> {
    let mut salt = [0_u8; PASSWORD_SALT_LEN];
    SystemRandom::new()
        .fill(&mut salt)
        .map_err(|_| io::Error::other("Failed to generate password salt"))?;
    let wrapping_key = derive_password_key(password, &salt)?;
    let wrapped = encrypt_with_key(&wrapping_key, data_key)?;
    set_metadata(connection, META_WRAPPED_KEY_SALT, &BASE64.encode(salt))?;
    set_metadata(
        connection,
        META_WRAPPED_KEY_NONCE,
        &BASE64.encode(wrapped.nonce),
    )?;
    set_metadata(
        connection,
        META_WRAPPED_KEY_CIPHERTEXT,
        &BASE64.encode(wrapped.ciphertext),
    )?;
    Ok(())
}

fn derive_password_key(password: &str, salt: &[u8]) -> io::Result<[u8; 32]> {
    let mut derived = [0_u8; 32];
    let iterations = NonZeroU32::new(KDF_ITERATIONS).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Invalid PBKDF2 iteration count",
        )
    })?;
    pbkdf2::derive(
        pbkdf2::PBKDF2_HMAC_SHA256,
        iterations,
        salt,
        password.as_bytes(),
        &mut derived,
    );
    Ok(derived)
}

fn decode_metadata_base64(connection: &Connection, key: &str) -> io::Result<Vec<u8>> {
    let value = get_metadata(connection, key)?.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("Missing security metadata: {key}"),
        )
    })?;
    BASE64
        .decode(value)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))
}

fn database_path() -> io::Result<PathBuf> {
    Ok(storage_dir()?.join(DATABASE_FILE))
}

fn persisted_storage_requires_existing_key() -> io::Result<bool> {
    let db_path = database_path()?;
    if !db_path.exists() {
        return Ok(false);
    }

    let connection = open_connection_for_read()?;
    let encrypted_entry_count: i64 = connection
        .query_row("SELECT COUNT(*) FROM encrypted_entries", [], |row| row.get(0))
        .map_err(to_io_error)?;
    if encrypted_entry_count > 0 {
        return Ok(true);
    }

    let has_wrapped_key = get_metadata(&connection, META_WRAPPED_KEY_CIPHERTEXT)?.is_some();
    Ok(has_wrapped_key)
}

fn validate_storage_key_against_existing_data(storage_key: &[u8; 32]) -> io::Result<()> {
    let connection = open_connection_for_read()?;
    probe_existing_entry_decryption(&connection, storage_key)
}

fn probe_existing_state_decryption(
    connection: &Connection,
    storage_key: &[u8; 32],
) -> io::Result<()> {
    let row = connection
        .query_row(
            "SELECT nonce, ciphertext
             FROM encrypted_entries
             LIMIT 1",
            [],
            |row| {
                Ok(EncryptedPayload {
                    nonce: row.get(0)?,
                    ciphertext: row.get(1)?,
                })
            },
        )
        .optional()
        .map_err(to_io_error)?;

    let Some(payload) = row else {
        return Ok(());
    };

    decrypt_with_key(storage_key, &payload).map(|_| ())
}

fn probe_existing_entry_decryption(
    connection: &Connection,
    storage_key: &[u8; 32],
) -> io::Result<()> {
    let row = connection
        .query_row(
            "SELECT nonce, ciphertext
             FROM encrypted_entries
             LIMIT 1",
            [],
            |row| {
                Ok(EncryptedPayload {
                    nonce: row.get(0)?,
                    ciphertext: row.get(1)?,
                })
            },
        )
        .optional()
        .map_err(to_io_error)?;

    let Some(payload) = row else {
        return Ok(());
    };

    decrypt_with_key(storage_key, &payload).map(|_| ())
}

fn recovery_dir() -> io::Result<PathBuf> {
    Ok(storage_dir()?.join(RECOVERY_DIR_NAME))
}

fn export_dir() -> io::Result<PathBuf> {
    Ok(storage_dir()?.join(BACKUP_DIR_NAME).join(EXPORT_DIR_NAME))
}

fn recovery_key_path() -> io::Result<PathBuf> {
    Ok(recovery_dir()?.join(RECOVERY_KEY_FILE))
}

fn write_recovery_export(
    accounts: &[StoredAccountState],
    drafts: &[DraftMessage],
    contacts: &PersistedContacts,
) -> io::Result<()> {
    let export_dir = export_dir()?;
    fs::create_dir_all(&export_dir)?;

    let mut export_accounts = accounts.to_vec();
    hydrate_accounts_with_keyring_secrets(&mut export_accounts, false);

    let export = RecoveryExport {
        version: 1,
        exported_at: Utc::now().to_rfc3339(),
        accounts: export_accounts,
        drafts: drafts.to_vec(),
        contacts: contacts.clone(),
    };
    let payload = serde_json::to_vec_pretty(&export)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))?;

    let latest_path = export_dir.join("latest.json");
    let temp_latest_path = export_dir.join("latest.json.tmp");
    fs::write(&temp_latest_path, &payload)?;
    apply_owner_only_permissions(&temp_latest_path)?;
    fs::rename(&temp_latest_path, &latest_path)?;

    let timestamp = Utc::now().format("%Y%m%d-%H%M%S-%3f");
    let snapshot_path = export_dir.join(format!("recovery-{timestamp}.json"));
    fs::write(&snapshot_path, &payload)?;
    apply_owner_only_permissions(&snapshot_path)?;
    prune_recovery_exports(&export_dir)?;
    Ok(())
}

fn prune_recovery_exports(export_dir: &Path) -> io::Result<()> {
    let mut exports = fs::read_dir(export_dir)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_name().to_string_lossy().starts_with("recovery-"))
        .filter(|entry| entry.path().extension().and_then(|ext| ext.to_str()) == Some("json"))
        .collect::<Vec<_>>();

    exports.sort_by_key(|entry| entry.file_name());
    let stale_count = exports.len().saturating_sub(MAX_RECOVERY_EXPORTS);
    for entry in exports.into_iter().take(stale_count) {
        let _ = fs::remove_file(entry.path());
    }

    Ok(())
}

fn persist_recovery_key_file(key: &[u8; 32]) -> io::Result<()> {
    let recovery_dir = recovery_dir()?;
    fs::create_dir_all(&recovery_dir)?;
    let path = recovery_key_path()?;
    let temp_path = path.with_extension("tmp");
    fs::write(&temp_path, BASE64.encode(key))?;
    apply_owner_only_permissions(&temp_path)?;
    fs::rename(temp_path, path)?;
    Ok(())
}

fn load_recovery_key_file() -> io::Result<Option<[u8; 32]>> {
    let path = recovery_key_path()?;
    if !path.exists() {
        return Ok(None);
    }

    let encoded = fs::read_to_string(path)?;
    decode_storage_key(encoded.trim()).map(Some)
}

fn load_latest_recovery_export() -> io::Result<RecoveryExport> {
    let path = latest_recovery_export_path()?;
    let contents = fs::read_to_string(&path)?;
    serde_json::from_str::<RecoveryExport>(&contents).map_err(|error| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "Failed to parse recovery export {}: {}",
                path.display(),
                error
            ),
        )
    })
}

fn latest_recovery_export_path() -> io::Result<PathBuf> {
    let export_dir = export_dir()?;
    fs::create_dir_all(&export_dir)?;

    let latest_path = export_dir.join("latest.json");
    if latest_path.exists() {
        return Ok(latest_path);
    }

    let mut snapshots = fs::read_dir(&export_dir)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_name().to_string_lossy().starts_with("recovery-"))
        .filter(|entry| entry.path().extension().and_then(|ext| ext.to_str()) == Some("json"))
        .collect::<Vec<_>>();
    snapshots.sort_by_key(|entry| entry.file_name());
    snapshots
        .pop()
        .map(|entry| entry.path())
        .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "No recovery export available"))
}

fn archive_storage_dir(prefix: &str) -> io::Result<()> {
    let storage_path = storage_dir()?;
    let archive_root = data_root()?.join(APP_DIR_NAME).join(RESET_ARCHIVE_DIR_NAME);
    fs::create_dir_all(&archive_root)?;

    if !storage_path.exists() {
        return Ok(());
    }

    let timestamp = Utc::now().format("%Y%m%d-%H%M%S-%3f");
    let archive_path = archive_root.join(format!("{prefix}-{timestamp}"));
    fs::rename(&storage_path, &archive_path)?;
    eprintln!(
        "[store] archived local encrypted storage to {}",
        archive_path.display()
    );
    Ok(())
}

#[cfg(unix)]
fn apply_owner_only_permissions(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
}

#[cfg(not(unix))]
fn apply_owner_only_permissions(_path: &Path) -> io::Result<()> {
    Ok(())
}

fn legacy_load_json<T: DeserializeOwned>(path: &Path) -> io::Result<T> {
    let contents = fs::read_to_string(path)?;
    serde_json::from_str(&contents)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))
}

fn legacy_load_accounts(path: &Path) -> io::Result<Vec<StoredAccountState>> {
    let mut accounts: Vec<StoredAccountState> = legacy_load_json(path)?;
    hydrate_accounts_with_keyring_secrets(&mut accounts, false);

    Ok(accounts)
}

fn legacy_raw_path(message_id: &str) -> io::Result<PathBuf> {
    Ok(storage_dir()?
        .join("raw")
        .join(format!("{}.eml", sanitize_id(message_id))))
}

fn load_legacy_avatar(contact_id: &str) -> io::Result<Option<(String, Vec<u8>)>> {
    let Some(path) = legacy_avatar_path(contact_id)? else {
        return Ok(None);
    };
    let payload = fs::read(path)?;
    Ok(Some(("image/webp".into(), payload)))
}

fn legacy_avatar_path(contact_id: &str) -> io::Result<Option<PathBuf>> {
    let path = storage_dir()?
        .join("avatars")
        .join(format!("{contact_id}.webp"));
    if path.exists() {
        Ok(Some(path))
    } else {
        Ok(None)
    }
}

fn sanitize_id(id: &str) -> String {
    id.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_")
}

fn storage_dir() -> io::Result<PathBuf> {
    let base = data_root()?;
    Ok(base.join(APP_DIR_NAME).join(STORAGE_DIR_NAME))
}

fn data_root() -> io::Result<PathBuf> {
    if let Ok(value) = env::var("MAILYOU_DATA_DIR") {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return Ok(PathBuf::from(trimmed));
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(value) = env::var("APPDATA") {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return Ok(PathBuf::from(trimmed));
            }
        }
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "APPDATA environment variable not set",
        ));
    }

    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(value) = env::var("XDG_DATA_HOME") {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return Ok(PathBuf::from(trimmed));
            }
        }

        let home = env::var("HOME")
            .map(PathBuf::from)
            .map_err(|error| io::Error::new(io::ErrorKind::NotFound, error.to_string()))?;
        Ok(home.join(".local").join("share"))
    }
}

fn keyring_get(account_id: &str, secret_kind: &str) -> Option<String> {
    keyring::Entry::new(KEYRING_SERVICE, &format!("{account_id}:{secret_kind}"))
        .ok()
        .and_then(|entry| entry.get_password().ok())
}

fn keyring_set(account_id: &str, secret_kind: &str, secret_value: &str) -> bool {
    keyring::Entry::new(KEYRING_SERVICE, &format!("{account_id}:{secret_kind}"))
        .ok()
        .and_then(|entry| entry.set_password(secret_value).ok())
        .is_some()
}

fn keyring_delete(account_id: &str, secret_kind: &str) -> bool {
    keyring::Entry::new(KEYRING_SERVICE, &format!("{account_id}:{secret_kind}"))
        .ok()
        .and_then(|entry| entry.delete_credential().ok())
        .is_some()
}

fn probe_system_keyring() -> (bool, Option<String>) {
    let secret = format!("probe-{}", std::process::id());
    let entry = match keyring::Entry::new(
        KEYRING_SERVICE,
        &format!("{STORAGE_PROBE_ACCOUNT}:{STORAGE_PROBE_KIND}"),
    ) {
        Ok(entry) => entry,
        Err(error) => return (false, Some(error.to_string())),
    };

    if let Err(error) = entry.set_password(&secret) {
        return (false, Some(error.to_string()));
    }

    let _ = entry.delete_credential();
    (true, None)
}

fn to_io_error(error: rusqlite::Error) -> io::Error {
    io::Error::other(error.to_string())
}

fn ensure_database_backup(connection: &Connection) -> io::Result<()> {
    if session_backup_created().load(Ordering::Relaxed) {
        return Ok(());
    }

    let db_path = database_path()?;
    if !db_path.exists() {
        session_backup_created().store(true, Ordering::Relaxed);
        return Ok(());
    }

    let backup_dir = storage_dir()?.join(BACKUP_DIR_NAME);
    fs::create_dir_all(&backup_dir)?;
    let timestamp = Utc::now().format("%Y%m%d-%H%M%S-%3f");
    let backup_path = backup_dir.join(format!("storage-{timestamp}.sqlite3"));
    let backup_sql = format!("VACUUM INTO '{}'", backup_path.to_string_lossy().replace('\'', "''"));
    connection.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);").map_err(to_io_error)?;
    connection.execute_batch(&backup_sql).map_err(to_io_error)?;
    prune_database_backups(&backup_dir)?;
    session_backup_created().store(true, Ordering::Relaxed);
    eprintln!("[store] created backup {}", backup_path.display());
    Ok(())
}

fn prune_database_backups(backup_dir: &Path) -> io::Result<()> {
    let mut backups = fs::read_dir(backup_dir)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().and_then(|ext| ext.to_str()) == Some("sqlite3"))
        .collect::<Vec<_>>();

    backups.sort_by_key(|entry| entry.file_name());
    let stale_count = backups.len().saturating_sub(MAX_DATABASE_BACKUPS);
    for entry in backups.into_iter().take(stale_count) {
        let _ = fs::remove_file(entry.path());
    }

    Ok(())
}
