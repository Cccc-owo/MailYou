use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use crate::models::{
    AccountSetupDraft, AccountStatus, DraftMessage, MailAccount, MailFolderKind, MailMessage,
    MailThread, MailboxBundle, MailboxFolder, StoredAccountState, SyncStatus,
};
use crate::protocol::BackendError;
use crate::storage::{accounts, drafts, mailbox, persisted, sync};

pub fn backend_name() -> &'static str {
    "persisted-mock"
}

pub fn list_accounts() -> Result<Vec<MailAccount>, BackendError> {
    Ok(state().lock().unwrap().accounts())
}

pub fn test_account_connection(draft: AccountSetupDraft) -> Result<SyncStatus, BackendError> {
    validate_account_draft(&draft)?;

    let host = draft.incoming_host.trim().to_lowercase();
    if host.contains("fail") || host.contains("invalid") || draft.password.trim().is_empty() {
        return Err(BackendError::validation(
            "Unable to connect with the provided IMAP settings",
        ));
    }

    Ok(SyncStatus {
        account_id: "connection-test".into(),
        state: "idle".into(),
        message: format!(
            "Connected to {}:{} and {}:{}",
            draft.incoming_host, draft.incoming_port, draft.outgoing_host, draft.outgoing_port
        ),
        updated_at: current_timestamp(),
    })
}

pub fn create_account(draft: AccountSetupDraft) -> Result<MailAccount, BackendError> {
    test_account_connection(draft.clone())?;

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
    let last_synced_at = current_timestamp();
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
    state.persist()?;

    Ok(account)
}

pub fn list_folders(account_id: &str) -> Result<Vec<MailboxFolder>, BackendError> {
    Ok(state()
        .lock()
        .unwrap()
        .folders
        .iter()
        .filter(|folder| folder.account_id == account_id)
        .cloned()
        .collect())
}

pub fn list_messages(account_id: &str, folder_id: &str) -> Result<Vec<MailMessage>, BackendError> {
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
    Ok(messages)
}

pub fn get_message(account_id: &str, message_id: &str) -> Result<Option<MailMessage>, BackendError> {
    Ok(state()
        .lock()
        .unwrap()
        .messages
        .iter()
        .find(|message| message.account_id == account_id && message.id == message_id)
        .cloned())
}

pub fn save_draft(draft: DraftMessage) -> Result<DraftMessage, BackendError> {
    let mut state = state().lock().unwrap();

    if let Some(existing) = state.drafts.iter_mut().find(|item| item.id == draft.id) {
        *existing = draft.clone();
        let updated = existing.clone();
        state.persist()?;
        return Ok(updated);
    }

    let next_draft = DraftMessage {
        id: if draft.id.trim().is_empty() {
            format!("draft-{}", state.drafts.len() + 1)
        } else {
            draft.id.clone()
        },
        ..draft
    };

    sync_draft_mailbox_message(&mut state, &next_draft);
    state.drafts.insert(0, next_draft.clone());
    state.persist()?;
    Ok(next_draft)
}

pub fn send_message(draft: DraftMessage) -> Result<String, BackendError> {
    if draft.account_id.trim().is_empty() || draft.to.trim().is_empty() {
        return Err(BackendError::validation("Recipient and account are required"));
    }

    let mut state = state().lock().unwrap();
    state.drafts.retain(|item| item.id != draft.id);
    state.messages.retain(|message| message.id != draft.id);

    let sent_folder = state
        .folders
        .iter()
        .find(|folder| folder.account_id == draft.account_id && matches!(folder.kind, MailFolderKind::Sent))
        .map(|folder| folder.id.clone())
        .ok_or_else(|| BackendError::internal("Sent folder is missing"))?;

    let thread_id = format!("thread-sent-{}", state.messages.len() + 1);
    let timestamp = current_timestamp();
    let account = state
        .account_states
        .iter()
        .find(|account_state| account_state.account.id == draft.account_id)
        .map(|state| state.account.clone())
        .ok_or_else(|| BackendError::not_found("Account not found"))?;

    let message = MailMessage {
        id: format!("sent-{}", state.messages.len() + 1),
        account_id: draft.account_id.clone(),
        folder_id: sent_folder,
        thread_id: thread_id.clone(),
        subject: draft.subject.clone(),
        preview: preview_for(&draft.body),
        body: draft.body.clone(),
        from: account.name,
        from_email: account.email,
        to: split_recipients(&draft.to),
        cc: split_recipients(&draft.cc),
        sent_at: timestamp.clone(),
        received_at: timestamp.clone(),
        is_read: true,
        is_starred: false,
        has_attachments: false,
        attachments: vec![],
        labels: vec!["Sent".into()],
    };

    state.threads.insert(
        0,
        MailThread {
            id: thread_id,
            account_id: draft.account_id.clone(),
            subject: draft.subject.clone(),
            message_ids: vec![message.id.clone()],
            last_message_at: timestamp.clone(),
            unread_count: 0,
        },
    );
    state.messages.insert(0, message);
    state.sync_statuses.insert(
        draft.account_id.clone(),
        SyncStatus {
            account_id: draft.account_id,
            state: "idle".into(),
            message: "Message sent successfully".into(),
            updated_at: timestamp.clone(),
        },
    );
    state.persist()?;

    Ok(timestamp)
}

pub fn toggle_star(account_id: &str, message_id: &str) -> Result<Option<MailMessage>, BackendError> {
    let mut state = state().lock().unwrap();
    let updated = {
        let Some(message) = state
            .messages
            .iter_mut()
            .find(|message| message.account_id == account_id && message.id == message_id)
        else {
            return Ok(None);
        };

        message.is_starred = !message.is_starred;
        message.clone()
    };
    state.persist()?;
    Ok(Some(updated))
}

pub fn toggle_read(account_id: &str, message_id: &str) -> Result<Option<MailMessage>, BackendError> {
    let mut state = state().lock().unwrap();
    let updated = {
        let Some(message) = state
            .messages
            .iter_mut()
            .find(|message| message.account_id == account_id && message.id == message_id)
        else {
            return Ok(None);
        };

        message.is_read = !message.is_read;
        message.clone()
    };
    state.persist()?;
    Ok(Some(updated))
}

pub fn delete_message(account_id: &str, message_id: &str) -> Result<(), BackendError> {
    let mut state = state().lock().unwrap();
    let trash_folder_id = state
        .folders
        .iter()
        .find(|folder| folder.account_id == account_id && matches!(folder.kind, MailFolderKind::Trash))
        .map(|folder| folder.id.clone())
        .ok_or_else(|| BackendError::internal("Trash folder is missing"))?;

    let Some(message) = state
        .messages
        .iter_mut()
        .find(|message| message.account_id == account_id && message.id == message_id)
    else {
        return Ok(());
    };

    message.folder_id = trash_folder_id;
    message.is_read = true;
    state.persist()?;
    Ok(())
}

pub fn archive_message(account_id: &str, message_id: &str) -> Result<Option<MailMessage>, BackendError> {
    let mut state = state().lock().unwrap();
    let archive_folder_id = state
        .folders
        .iter()
        .find(|folder| folder.account_id == account_id && matches!(folder.kind, MailFolderKind::Archive))
        .map(|folder| folder.id.clone())
        .ok_or_else(|| BackendError::internal("Archive folder is missing"))?;

    let Some(message) = state
        .messages
        .iter_mut()
        .find(|message| message.account_id == account_id && message.id == message_id)
    else {
        return Ok(None);
    };

    message.folder_id = archive_folder_id;
    let updated = message.clone();
    state.persist()?;
    Ok(Some(updated))
}

pub fn restore_message(account_id: &str, message_id: &str) -> Result<Option<MailMessage>, BackendError> {
    let mut state = state().lock().unwrap();
    let inbox_folder_id = state
        .folders
        .iter()
        .find(|folder| folder.account_id == account_id && matches!(folder.kind, MailFolderKind::Inbox))
        .map(|folder| folder.id.clone())
        .ok_or_else(|| BackendError::internal("Inbox folder is missing"))?;

    let Some(message) = state
        .messages
        .iter_mut()
        .find(|message| message.account_id == account_id && message.id == message_id)
    else {
        return Ok(None);
    };

    message.folder_id = inbox_folder_id;
    let updated = message.clone();
    state.persist()?;
    Ok(Some(updated))
}

pub fn move_message(account_id: &str, message_id: &str, folder_id: &str) -> Result<Option<MailMessage>, BackendError> {
    let mut state = state().lock().unwrap();

    let folder_exists = state
        .folders
        .iter()
        .any(|folder| folder.account_id == account_id && folder.id == folder_id);

    if !folder_exists {
        return Err(BackendError::not_found("Target folder not found"));
    }

    let Some(message) = state
        .messages
        .iter_mut()
        .find(|message| message.account_id == account_id && message.id == message_id)
    else {
        return Ok(None);
    };

    message.folder_id = folder_id.to_string();
    let updated = message.clone();
    state.persist()?;
    Ok(Some(updated))
}

pub fn sync_account(account_id: &str) -> Result<SyncStatus, BackendError> {
    let mut state = state().lock().unwrap();
    if !state
        .account_states
        .iter()
        .any(|account_state| account_state.account.id == account_id)
    {
        return Err(BackendError::not_found("Account not found"));
    }

    let timestamp = current_timestamp();
    let status = SyncStatus {
        account_id: account_id.into(),
        state: "idle".into(),
        message: "Sync completed successfully".into(),
        updated_at: timestamp.clone(),
    };

    if let Some(account_state) = state
        .account_states
        .iter_mut()
        .find(|account_state| account_state.account.id == account_id)
    {
        account_state.account.last_synced_at = timestamp;
        account_state.account.status = AccountStatus::Connected;
    }

    state.sync_statuses.insert(account_id.to_string(), status.clone());
    state.persist()?;

    Ok(status)
}

pub fn get_mailbox_bundle(account_id: &str) -> Result<MailboxBundle, BackendError> {
    let state = state().lock().unwrap();

    Ok(MailboxBundle {
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
            .unwrap_or_else(|| sync::initial_sync_status(account_id, &current_timestamp())),
    })
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

    fn persist(&self) -> Result<(), BackendError> {
        persisted::save_accounts(&self.account_states)
            .and_then(|_| persisted::save_drafts(&self.drafts))
            .and_then(|_| {
                persisted::save_mailbox(&persisted::PersistedMailbox {
                    folders: self.folders.clone(),
                    messages: self.messages.clone(),
                    threads: self.threads.clone(),
                })
            })
            .and_then(|_| {
                let statuses: Vec<SyncStatus> = self.sync_statuses.values().cloned().collect();
                persisted::save_sync_statuses(&statuses)
            })
            .map_err(|error| BackendError::internal(error.to_string()))
    }
}

fn state() -> &'static Mutex<MemoryState> {
    static STATE: OnceLock<Mutex<MemoryState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(initial_state()))
}

fn initial_state() -> MemoryState {
    let seeded_accounts = accounts::seeded_account_states();
    let persisted_accounts = persisted::load_accounts();
    let account_states = if persisted_accounts.is_empty() {
        seeded_accounts
    } else {
        persisted_accounts
    };

    let seeded_mailbox = persisted::PersistedMailbox {
        folders: mailbox::seeded_folders(),
        messages: mailbox::seeded_messages(),
        threads: mailbox::seeded_threads(),
    };
    let mailbox_state = {
        let loaded = persisted::load_mailbox();
        if loaded.folders.is_empty() && loaded.messages.is_empty() && loaded.threads.is_empty() {
            seeded_mailbox
        } else {
            loaded
        }
    };

    let seeded_drafts = drafts::seeded_drafts();
    let drafts = {
        let loaded = persisted::load_drafts();
        if loaded.is_empty() {
            seeded_drafts
        } else {
            loaded
        }
    };

    let sync_statuses = {
        let loaded = persisted::load_sync_statuses();
        if loaded.is_empty() {
            sync::seeded_sync_statuses()
        } else {
            loaded
                .into_iter()
                .map(|status| (status.account_id.clone(), status))
                .collect()
        }
    };

    let mut state = MemoryState {
        account_states,
        folders: mailbox_state.folders,
        messages: mailbox_state.messages,
        threads: mailbox_state.threads,
        drafts,
        sync_statuses,
    };

    state.sync_drafts_into_mailbox();
    let _ = state.persist();
    state
}

impl MemoryState {
    fn sync_drafts_into_mailbox(&mut self) {
        for draft in self.drafts.clone() {
            sync_draft_mailbox_message(self, &draft);
        }
    }
}

fn sync_draft_mailbox_message(state: &mut MemoryState, draft: &DraftMessage) {
    let Some(folder_id) = state
        .folders
        .iter()
        .find(|folder| folder.account_id == draft.account_id && matches!(folder.kind, MailFolderKind::Drafts))
        .map(|folder| folder.id.clone())
    else {
        return;
    };

    let preview = preview_for(&draft.body);
    if let Some(message) = state.messages.iter_mut().find(|message| message.id == draft.id) {
        message.folder_id = folder_id;
        message.subject = draft.subject.clone();
        message.preview = preview.clone();
        message.body = draft.body.clone();
        message.to = split_recipients(&draft.to);
        message.cc = split_recipients(&draft.cc);
        message.received_at = current_timestamp();
        return;
    }

    state.messages.insert(
        0,
        MailMessage {
            id: draft.id.clone(),
            account_id: draft.account_id.clone(),
            folder_id,
            thread_id: format!("thread-draft-{}", draft.id),
            subject: draft.subject.clone(),
            preview,
            body: draft.body.clone(),
            from: "Draft".into(),
            from_email: "draft@local".into(),
            to: split_recipients(&draft.to),
            cc: split_recipients(&draft.cc),
            sent_at: current_timestamp(),
            received_at: current_timestamp(),
            is_read: true,
            is_starred: false,
            has_attachments: false,
            attachments: vec![],
            labels: vec!["Draft".into()],
        },
    );
}

fn validate_account_draft(draft: &AccountSetupDraft) -> Result<(), BackendError> {
    if draft.email.trim().is_empty()
        || draft.incoming_host.trim().is_empty()
        || draft.outgoing_host.trim().is_empty()
        || draft.username.trim().is_empty()
    {
        return Err(BackendError::validation("All account fields are required"));
    }

    if draft.incoming_port == 0 || draft.outgoing_port == 0 {
        return Err(BackendError::validation("Ports must be greater than 0"));
    }

    Ok(())
}

fn preview_for(body: &str) -> String {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return "(No message body)".into();
    }

    trimmed.chars().take(96).collect()
}

fn split_recipients(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn current_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    format!("{seconds}Z")
}
