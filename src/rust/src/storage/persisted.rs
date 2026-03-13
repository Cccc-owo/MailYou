use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;

use serde::{de::DeserializeOwned, Serialize};

use crate::models::{DraftMessage, StoredAccountState, SyncStatus};

const APP_DIR_NAME: &str = "MailYou";
const STORAGE_DIR_NAME: &str = "mail";
const ACCOUNTS_FILE: &str = "accounts.json";
const DRAFTS_FILE: &str = "drafts.json";
const MAILBOX_FILE: &str = "mailbox.json";
const SYNC_FILE: &str = "sync.json";

const KEYRING_SERVICE: &str = "mailyou";
const PASSWORD_PLACEHOLDER: &str = "<keyring>";

pub fn load_accounts() -> Vec<StoredAccountState> {
    let mut accounts: Vec<StoredAccountState> = load_json(ACCOUNTS_FILE).unwrap_or_default();

    for account_state in accounts.iter_mut() {
        if account_state.config.password == PASSWORD_PLACEHOLDER
            || account_state.config.password.is_empty()
        {
            if let Some(password) = keyring_get(&account_state.account.id) {
                account_state.config.password = password;
            }
        }
    }

    accounts
}

pub fn save_accounts(accounts: &[StoredAccountState]) -> io::Result<()> {
    // Store passwords in keyring, replace with placeholder in JSON
    let mut sanitized: Vec<StoredAccountState> = accounts.to_vec();

    for account_state in sanitized.iter_mut() {
        let account_id = &account_state.account.id;
        let password = &account_state.config.password;

        if !password.is_empty() && password != PASSWORD_PLACEHOLDER {
            if keyring_set(account_id, password) {
                account_state.config.password = PASSWORD_PLACEHOLDER.into();
            }
            // If keyring fails, password stays in JSON as fallback
        }
    }

    save_json(ACCOUNTS_FILE, &sanitized)
}

pub fn load_drafts() -> Vec<DraftMessage> {
    load_json(DRAFTS_FILE).unwrap_or_default()
}

pub fn save_drafts(drafts: &[DraftMessage]) -> io::Result<()> {
    save_json(DRAFTS_FILE, drafts)
}

pub fn load_mailbox() -> PersistedMailbox {
    load_json(MAILBOX_FILE).unwrap_or_default()
}

pub fn save_mailbox(mailbox: &PersistedMailbox) -> io::Result<()> {
    save_json(MAILBOX_FILE, mailbox)
}

pub fn load_sync_statuses() -> Vec<SyncStatus> {
    load_json(SYNC_FILE).unwrap_or_default()
}

pub fn save_sync_statuses(statuses: &[SyncStatus]) -> io::Result<()> {
    save_json(SYNC_FILE, statuses)
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersistedMailbox {
    pub folders: Vec<crate::models::MailboxFolder>,
    pub messages: Vec<crate::models::MailMessage>,
    pub threads: Vec<crate::models::MailThread>,
}

fn load_json<T: DeserializeOwned>(file_name: &str) -> io::Result<T> {
    let path = storage_file(file_name)?;
    let contents = fs::read_to_string(path)?;
    serde_json::from_str(&contents)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))
}

pub fn has_persisted(file_name: &str) -> bool {
    storage_file(file_name).map(|p| p.exists()).unwrap_or(false)
}

pub fn has_accounts_file() -> bool {
    has_persisted(ACCOUNTS_FILE)
}

pub fn has_mailbox_file() -> bool {
    has_persisted(MAILBOX_FILE)
}

pub fn has_drafts_file() -> bool {
    has_persisted(DRAFTS_FILE)
}

fn save_json<T: Serialize + ?Sized>(file_name: &str, value: &T) -> io::Result<()> {
    let path = storage_file(file_name)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let payload = serde_json::to_string_pretty(value)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error.to_string()))?;
    fs::write(path, payload)
}

fn storage_file(file_name: &str) -> io::Result<PathBuf> {
    Ok(storage_dir()?.join(file_name))
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

pub fn save_raw_email(message_id: &str, raw: &[u8]) -> io::Result<()> {
    let dir = storage_dir()?.join("raw");
    fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{}.eml", sanitize_id(message_id)));
    fs::write(path, raw)
}

pub fn load_raw_email(message_id: &str) -> io::Result<Vec<u8>> {
    let path = storage_dir()?.join("raw").join(format!("{}.eml", sanitize_id(message_id)));
    fs::read(path)
}

fn sanitize_id(id: &str) -> String {
    id.replace(['/', '\\', ':', '*', '?', '"', '<', '>', '|'], "_")
}

fn keyring_get(account_id: &str) -> Option<String> {
    keyring::Entry::new(KEYRING_SERVICE, account_id)
        .ok()
        .and_then(|entry| entry.get_password().ok())
}

fn keyring_set(account_id: &str, password: &str) -> bool {
    keyring::Entry::new(KEYRING_SERVICE, account_id)
        .ok()
        .and_then(|entry| entry.set_password(password).ok())
        .is_some()
}
