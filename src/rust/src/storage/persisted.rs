use std::env;
use std::fs;
use std::io;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};
use ring::pbkdf2;
use ring::rand::{SecureRandom, SystemRandom};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{de::DeserializeOwned, Serialize};

use crate::models::{Contact, ContactGroup, DraftMessage, StoredAccountState, SyncStatus};
use crate::protocol::StorageSecurityStatus;

const APP_DIR_NAME: &str = "MailYou";
const STORAGE_DIR_NAME: &str = "mail";
const DATABASE_FILE: &str = "storage.sqlite3";
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

const MODE_KEYRING: &str = "keyring";
const MODE_PASSWORD: &str = "password";

const STATE_ACCOUNTS: &str = "accounts";
const STATE_DRAFTS: &str = "drafts";
const STATE_MAILBOX: &str = "mailbox";
const STATE_SYNC: &str = "sync";
const STATE_CONTACTS: &str = "contacts";

#[derive(Debug, Clone)]
struct EncryptedPayload {
    nonce: Vec<u8>,
    ciphertext: Vec<u8>,
}

fn unlocked_storage_key() -> &'static Mutex<Option<[u8; 32]>> {
    static STORAGE_KEY: OnceLock<Mutex<Option<[u8; 32]>>> = OnceLock::new();
    STORAGE_KEY.get_or_init(|| Mutex::new(None))
}

pub fn load_accounts() -> Vec<StoredAccountState> {
    let mut accounts: Vec<StoredAccountState> = match load_state_json(STATE_ACCOUNTS) {
        Ok(accounts) => accounts,
        Err(error) if error.kind() == io::ErrorKind::PermissionDenied => {
            panic!("storage locked: {}", error)
        }
        Err(_) => Vec::new(),
    };

    for account_state in &mut accounts {
        if account_state.config.password == SECRET_PLACEHOLDER
            || account_state.config.password.is_empty()
        {
            if let Some(password) = keyring_get(&account_state.account.id, "password") {
                account_state.config.password = password;
            }
        }

        if account_state.config.refresh_token == SECRET_PLACEHOLDER
            || account_state.config.refresh_token.is_empty()
        {
            if let Some(refresh_token) = keyring_get(&account_state.account.id, "refreshToken") {
                account_state.config.refresh_token = refresh_token;
            }
        }
    }

    accounts
}

pub fn save_accounts(accounts: &[StoredAccountState]) -> io::Result<()> {
    let mut sanitized = accounts.to_vec();

    for account_state in &mut sanitized {
        let account_id = &account_state.account.id;
        let password = &account_state.config.password;

        if !password.is_empty() && password != SECRET_PLACEHOLDER && keyring_set(account_id, "password", password) {
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

    save_state_json(STATE_ACCOUNTS, &sanitized)
}

pub fn load_drafts() -> Vec<DraftMessage> {
    match load_state_json(STATE_DRAFTS) {
        Ok(drafts) => drafts,
        Err(error) if error.kind() == io::ErrorKind::PermissionDenied => {
            panic!("storage locked: {}", error)
        }
        Err(_) => Vec::new(),
    }
}

pub fn save_drafts(drafts: &[DraftMessage]) -> io::Result<()> {
    save_state_json(STATE_DRAFTS, drafts)
}

pub fn load_mailbox() -> PersistedMailbox {
    match load_state_json(STATE_MAILBOX) {
        Ok(mailbox) => mailbox,
        Err(error) if error.kind() == io::ErrorKind::PermissionDenied => {
            panic!("storage locked: {}", error)
        }
        Err(_) => PersistedMailbox::default(),
    }
}

pub fn save_mailbox(mailbox: &PersistedMailbox) -> io::Result<()> {
    save_state_json(STATE_MAILBOX, mailbox)
}

pub fn load_sync_statuses() -> Vec<SyncStatus> {
    match load_state_json(STATE_SYNC) {
        Ok(statuses) => statuses,
        Err(error) if error.kind() == io::ErrorKind::PermissionDenied => {
            panic!("storage locked: {}", error)
        }
        Err(_) => Vec::new(),
    }
}

pub fn save_sync_statuses(statuses: &[SyncStatus]) -> io::Result<()> {
    save_state_json(STATE_SYNC, statuses)
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
        Err(error) if error.kind() == io::ErrorKind::PermissionDenied => {
            panic!("storage locked: {}", error)
        }
        Err(_) => PersistedContacts::default(),
    };
    hydrate_contact_avatar_tokens(&mut data.contacts);
    data
}

pub fn save_contacts(data: &PersistedContacts) -> io::Result<()> {
    save_state_json(STATE_CONTACTS, data)
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
    let is_unlocked = matches!(mode, MODE_KEYRING) || unlocked_storage_key().lock().unwrap().is_some();
    let (keyring_available, keyring_error) = probe_system_keyring();

    Ok(StorageSecurityStatus {
        has_master_password: matches!(mode, MODE_PASSWORD),
        is_unlocked,
        mode,
        keyring_available,
        keyring_error,
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
    serde_json::from_slice(&plaintext)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))
}

fn save_state_json<T: Serialize + ?Sized>(key: &str, value: &T) -> io::Result<()> {
    let plaintext = serde_json::to_vec(value)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))?;
    let payload = encrypt_payload(&plaintext)?;
    write_entry("state", key, None, &payload)
}

fn has_state(key: &str) -> bool {
    ensure_initialized()
        .and_then(|_| read_entry("state", key).map(|entry| entry.is_some()))
        .unwrap_or(false)
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
    Ok(Some((mime_type.unwrap_or_else(|| "application/octet-stream".into()), plaintext)))
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
    ensure_initialized()?;
    let connection = open_connection()?;
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
    let connection = open_connection()?;
    initialize_schema(&connection)?;
    migrate_legacy_state_if_needed(&connection)?;
    Ok(())
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
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Failed to decrypt stored payload"))?;

    Ok(plaintext.to_vec())
}

fn storage_key() -> io::Result<[u8; 32]> {
    let connection = open_connection()?;
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
        return decode_storage_key(&encoded);
    }

    let mut key = [0_u8; 32];
    SystemRandom::new()
        .fill(&mut key)
        .map_err(|_| io::Error::other("Failed to generate storage key"))?;
    store_data_key_in_keyring(&key)?;
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
    set_metadata(connection, META_WRAPPED_KEY_NONCE, &BASE64.encode(wrapped.nonce))?;
    set_metadata(connection, META_WRAPPED_KEY_CIPHERTEXT, &BASE64.encode(wrapped.ciphertext))?;
    Ok(())
}

fn derive_password_key(password: &str, salt: &[u8]) -> io::Result<[u8; 32]> {
    let mut derived = [0_u8; 32];
    let iterations = NonZeroU32::new(KDF_ITERATIONS)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Invalid PBKDF2 iteration count"))?;
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

fn legacy_load_json<T: DeserializeOwned>(path: &Path) -> io::Result<T> {
    let contents = fs::read_to_string(path)?;
    serde_json::from_str(&contents)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))
}

fn legacy_load_accounts(path: &Path) -> io::Result<Vec<StoredAccountState>> {
    let mut accounts: Vec<StoredAccountState> = legacy_load_json(path)?;

    for account_state in &mut accounts {
        if account_state.config.password == SECRET_PLACEHOLDER
            || account_state.config.password.is_empty()
        {
            if let Some(password) = keyring_get(&account_state.account.id, "password") {
                account_state.config.password = password;
            }
        }

        if account_state.config.refresh_token == SECRET_PLACEHOLDER
            || account_state.config.refresh_token.is_empty()
        {
            if let Some(refresh_token) = keyring_get(&account_state.account.id, "refreshToken") {
                account_state.config.refresh_token = refresh_token;
            }
        }
    }

    Ok(accounts)
}

fn legacy_raw_path(message_id: &str) -> io::Result<PathBuf> {
    Ok(storage_dir()?.join("raw").join(format!("{}.eml", sanitize_id(message_id))))
}

fn load_legacy_avatar(contact_id: &str) -> io::Result<Option<(String, Vec<u8>)>> {
    let Some(path) = legacy_avatar_path(contact_id)? else {
        return Ok(None);
    };
    let payload = fs::read(path)?;
    Ok(Some(("image/webp".into(), payload)))
}

fn legacy_avatar_path(contact_id: &str) -> io::Result<Option<PathBuf>> {
    let path = storage_dir()?.join("avatars").join(format!("{contact_id}.webp"));
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
