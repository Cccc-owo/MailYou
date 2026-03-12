use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use crate::models::{
    AccountSetupDraft, AccountStatus, DraftMessage, MailAccount, MailFolderKind, MailMessage,
    MailThread, MailboxBundle, MailboxFolder, StoredAccountState, SyncStatus,
};
use crate::storage::{accounts, drafts, mailbox, sync};

pub fn backend_name() -> &'static str {
    "memory-mock"
}

pub fn accounts() -> Vec<MailAccount> {
    state().lock().unwrap().accounts()
}

pub fn create_account(draft: AccountSetupDraft) -> MailAccount {
    let mut state = state().lock().unwrap();
    let display_name = draft.display_name.trim();
    let base_name = if display_name.is_empty() {
        draft.email.trim()
    } else {
        display_name
    };

    let initials = base_name
        .split_whitespace()
        .filter_map(|part| part.chars().next())
        .take(2)
        .map(|character| character.to_uppercase().collect::<String>())
        .collect::<String>();

    let account_id = state.unique_account_id(&draft.email);
    let last_synced_at = "2026-03-12T10:05:00.000Z".to_string();
    let config = accounts::config_from_draft(&draft);
    let account = MailAccount {
        id: account_id.clone(),
        name: base_name.to_string(),
        email: draft.email,
        provider: draft.provider,
        color: "#5B8DEF".into(),
        initials: if initials.is_empty() {
            "NA".into()
        } else {
            initials
        },
        unread_count: 0,
        status: AccountStatus::Connected,
        last_synced_at: last_synced_at.clone(),
    };

    state.insert_account_state(StoredAccountState {
        account: account.clone(),
        config,
    });
    state.folders.splice(0..0, mailbox::default_folders_for_account(&account_id));
    state.sync_statuses.insert(
        account_id.clone(),
        sync::initial_sync_status(&account_id, &last_synced_at),
    );

    account
}

pub fn folders(account_id: &str) -> Vec<MailboxFolder> {
    state()
        .lock()
        .unwrap()
        .folders
        .iter()
        .filter(|folder| folder.account_id == account_id)
        .cloned()
        .collect()
}

pub fn messages(account_id: &str, folder_id: &str) -> Vec<MailMessage> {
    let state = state().lock().unwrap();
    let folder = state
        .folders
        .iter()
        .find(|candidate| candidate.account_id == account_id && candidate.id == folder_id);

    let mut messages: Vec<MailMessage> = match folder {
        Some(folder) if matches!(folder.kind, MailFolderKind::Starred) => state
            .messages
            .iter()
            .filter(|message| message.account_id == account_id && message.is_starred)
            .cloned()
            .collect(),
        Some(_) => state
            .messages
            .iter()
            .filter(|message| message.account_id == account_id && message.folder_id == folder_id)
            .cloned()
            .collect(),
        None => Vec::new(),
    };

    messages.sort_by(|left, right| right.received_at.cmp(&left.received_at));
    messages
}

pub fn get_message(account_id: &str, message_id: &str) -> Option<MailMessage> {
    state()
        .lock()
        .unwrap()
        .messages
        .iter()
        .find(|message| message.account_id == account_id && message.id == message_id)
        .cloned()
}

pub fn save_draft(draft: DraftMessage) -> DraftMessage {
    let mut state = state().lock().unwrap();

    if let Some(existing) = state.drafts.iter_mut().find(|item| item.id == draft.id) {
        *existing = draft.clone();
        return existing.clone();
    }

    let next_draft = DraftMessage {
        id: if draft.id.trim().is_empty() {
            format!("draft-{}", state.drafts.len() + 1)
        } else {
            draft.id.clone()
        },
        ..draft
    };

    state.drafts.insert(0, next_draft.clone());
    next_draft
}

pub fn send_message(draft: DraftMessage) -> String {
    let mut state = state().lock().unwrap();
    state.drafts.retain(|item| item.id != draft.id);
    "2026-03-12T10:15:00.000Z".into()
}

pub fn toggle_star(account_id: &str, message_id: &str) -> Option<MailMessage> {
    let mut state = state().lock().unwrap();
    let message = state
        .messages
        .iter_mut()
        .find(|message| message.account_id == account_id && message.id == message_id)?;

    message.is_starred = !message.is_starred;
    Some(message.clone())
}

pub fn toggle_read(account_id: &str, message_id: &str) -> Option<MailMessage> {
    let mut state = state().lock().unwrap();
    let message = state
        .messages
        .iter_mut()
        .find(|message| message.account_id == account_id && message.id == message_id)?;

    message.is_read = !message.is_read;
    Some(message.clone())
}

pub fn delete_message(account_id: &str, message_id: &str) {
    let mut state = state().lock().unwrap();
    let trash_folder_id = state
        .folders
        .iter()
        .find(|folder| folder.account_id == account_id && matches!(folder.kind, MailFolderKind::Trash))
        .map(|folder| folder.id.clone());

    let Some(trash_folder_id) = trash_folder_id else {
        return;
    };

    let Some(message) = state
        .messages
        .iter_mut()
        .find(|message| message.account_id == account_id && message.id == message_id)
    else {
        return;
    };

    message.folder_id = trash_folder_id;
    message.is_read = true;
}

pub fn sync_status(account_id: &str) -> SyncStatus {
    let mut state = state().lock().unwrap();
    let status = SyncStatus {
        account_id: account_id.into(),
        state: "idle".into(),
        message: "Mailbox is up to date".into(),
        updated_at: "2026-03-12T10:00:00.000Z".into(),
    };

    state.sync_statuses.insert(account_id.to_string(), status.clone());

    status
}

pub fn mailbox_bundle(account_id: &str) -> MailboxBundle {
    let state = state().lock().unwrap();

    MailboxBundle {
        account_id: account_id.into(),
        folders: state
            .folders
            .iter()
            .filter(|folder| folder.account_id == account_id)
            .cloned()
            .collect(),
        messages: state
            .messages
            .iter()
            .filter(|message| message.account_id == account_id)
            .cloned()
            .collect(),
        threads: state
            .threads
            .iter()
            .filter(|thread| thread.account_id == account_id)
            .cloned()
            .collect(),
        sync_status: state
            .sync_statuses
            .get(account_id)
            .cloned()
            .unwrap_or_else(|| sync::initial_sync_status(account_id, "2026-03-12T10:00:00.000Z")),
    }
}

#[derive(Clone)]
struct MemoryState {
    account_states: Vec<StoredAccountState>,
    folders: Vec<MailboxFolder>,
    messages: Vec<MailMessage>,
    threads: Vec<MailThread>,
    drafts: Vec<DraftMessage>,
    sync_statuses: HashMap<String, SyncStatus>,
}

impl MemoryState {
    fn accounts(&self) -> Vec<MailAccount> {
        self.account_states
            .iter()
            .map(|state| state.account.clone())
            .collect()
    }

    fn insert_account_state(&mut self, account_state: StoredAccountState) {
        self.account_states.insert(0, account_state);
    }

    fn unique_account_id(&self, email: &str) -> String {
        let base = format!("acc-{}", email.trim().replace(['@', '.'], "-"));

        if !self
            .account_states
            .iter()
            .any(|account_state| account_state.account.id == base)
        {
            return base;
        }

        let mut suffix = 2;
        loop {
            let candidate = format!("{base}-{suffix}");
            if !self
                .account_states
                .iter()
                .any(|account_state| account_state.account.id == candidate)
            {
                return candidate;
            }
            suffix += 1;
        }
    }
}

fn state() -> &'static Mutex<MemoryState> {
    static STATE: OnceLock<Mutex<MemoryState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(initial_state()))
}

fn initial_state() -> MemoryState {
    let account_states = accounts::seeded_account_states();

    MemoryState {
        account_states,
        folders: mailbox::seeded_folders(),
        messages: mailbox::seeded_messages(),
        threads: mailbox::seeded_threads(),
        drafts: drafts::seeded_drafts(),
        sync_statuses: sync::seeded_sync_statuses(),
    }
}

