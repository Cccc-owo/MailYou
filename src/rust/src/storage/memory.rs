use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, OnceLock};

use crate::models::{
    AccountSetupDraft, AccountStatus, DraftMessage, MailAccount, MailFolderKind, MailMessage,
    MailThread, MailboxBundle, MailboxFolder, StoredAccountState, SyncStatus,
};
use crate::protocol::BackendError;
use crate::storage::{accounts, drafts, mailbox, persisted, sync};

/// Lock the global state, recovering from a poisoned Mutex if necessary.
/// A poisoned Mutex means a previous thread panicked while holding the lock.
/// The data may be inconsistent, but it's better to continue than to panic
/// (which would leave the request without a response).
fn lock_state() -> MutexGuard<'static, MemoryState> {
    state().lock().unwrap_or_else(|poisoned| {
        eprintln!("[backend] WARNING: recovering from poisoned Mutex");
        poisoned.into_inner()
    })
}

pub fn list_accounts() -> Result<Vec<MailAccount>, BackendError> {
    Ok(lock_state().accounts())
}

pub fn create_account_without_test(draft: AccountSetupDraft) -> Result<MailAccount, BackendError> {
    let mut state = lock_state();
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
    Ok(lock_state()
        .folders
        .iter()
        .filter(|folder| folder.account_id == account_id)
        .cloned()
        .collect())
}

pub fn list_messages(account_id: &str, folder_id: &str) -> Result<Vec<MailMessage>, BackendError> {
    let state = lock_state();
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
    Ok(lock_state()
        .messages
        .iter()
        .find(|message| message.account_id == account_id && message.id == message_id)
        .cloned())
}

pub fn save_draft(draft: DraftMessage) -> Result<DraftMessage, BackendError> {
    let mut state = lock_state();

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
    state.recalculate_counts();
    state.persist()?;
    Ok(next_draft)
}

pub fn toggle_star(account_id: &str, message_id: &str) -> Result<Option<MailMessage>, BackendError> {
    let mut state = lock_state();
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
    state.recalculate_counts();
    state.persist()?;
    Ok(Some(updated))
}

pub fn toggle_read(account_id: &str, message_id: &str) -> Result<Option<MailMessage>, BackendError> {
    let mut state = lock_state();
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
    state.recalculate_counts();
    state.persist()?;
    Ok(Some(updated))
}

pub fn delete_message(account_id: &str, message_id: &str) -> Result<(), BackendError> {
    let mut state = lock_state();
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
    state.recalculate_counts();
    state.persist()?;
    Ok(())
}

pub fn delete_account(account_id: &str) -> Result<(), BackendError> {
    let mut state = lock_state();

    if !state.account_states.iter().any(|s| s.account.id == account_id) {
        return Err(BackendError::not_found("Account not found"));
    }

    state.account_states.retain(|s| s.account.id != account_id);
    state.folders.retain(|f| f.account_id != account_id);
    state.messages.retain(|m| m.account_id != account_id);
    state.threads.retain(|t| t.account_id != account_id);
    state.drafts.retain(|d| d.account_id != account_id);
    state.sync_statuses.remove(account_id);
    state.persist()?;
    Ok(())
}

pub fn archive_message(account_id: &str, message_id: &str) -> Result<Option<MailMessage>, BackendError> {
    let mut state = lock_state();
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
    state.recalculate_counts();
    state.persist()?;
    Ok(Some(updated))
}

pub fn restore_message(account_id: &str, message_id: &str) -> Result<Option<MailMessage>, BackendError> {
    let mut state = lock_state();
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
    state.recalculate_counts();
    state.persist()?;
    Ok(Some(updated))
}

pub fn move_message(account_id: &str, message_id: &str, folder_id: &str) -> Result<Option<MailMessage>, BackendError> {
    let mut state = lock_state();

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
    state.recalculate_counts();
    state.persist()?;
    Ok(Some(updated))
}

pub fn mark_all_read(account_id: &str, folder_id: &str) -> Result<(), BackendError> {
    let mut state = lock_state();
    let folder = state
        .folders
        .iter()
        .find(|f| f.account_id == account_id && f.id == folder_id);

    let is_starred = folder.map(|f| matches!(f.kind, MailFolderKind::Starred)).unwrap_or(false);

    for message in state.messages.iter_mut() {
        if message.account_id != account_id {
            continue;
        }
        let in_folder = if is_starred {
            message.is_starred
        } else {
            message.folder_id == folder_id
        };
        if in_folder {
            message.is_read = true;
        }
    }

    state.recalculate_counts();
    state.persist()?;
    Ok(())
}

pub fn get_mailbox_bundle(account_id: &str) -> Result<MailboxBundle, BackendError> {
    let state = lock_state();

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

/// Return cached (body, preview) keyed by IMAP UID for all messages that
/// belong to the given account and already have a downloaded body.
/// Used by incremental sync to skip re-fetching bodies we already have.
pub fn get_existing_bodies(account_id: &str) -> std::collections::HashMap<u32, (String, String)> {
    let state = lock_state();
    let mut map = std::collections::HashMap::new();
    for msg in state.messages.iter().filter(|m| m.account_id == account_id) {
        if let Some(uid) = msg.imap_uid {
            if !msg.body.is_empty() {
                map.insert(uid, (msg.body.clone(), msg.preview.clone()));
            }
        }
    }
    map
}

pub fn get_account_state(account_id: &str) -> Option<StoredAccountState> {
    lock_state()
        .account_states
        .iter()
        .find(|s| s.account.id == account_id)
        .cloned()
}

pub fn record_sent_message(draft: DraftMessage) -> Result<String, BackendError> {
    let mut state = lock_state();
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
        .map(|s| s.account.clone())
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
        imap_uid: None,
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
    state.recalculate_counts();
    state.persist()?;
    Ok(timestamp)
}

pub fn merge_remote_mailbox(
    account_id: &str,
    folders: Vec<MailboxFolder>,
    remote_messages: Vec<MailMessage>,
    remote_threads: Vec<MailThread>,
) -> Result<(), BackendError> {
    let mut state = lock_state();

    // Folders: replace entirely (no local-only state worth preserving)
    state.folders.retain(|f| f.account_id != account_id);
    state.folders.extend(folders);

    // Messages: merge — preserve local is_read/is_starred/folder_id edits
    let local_by_id: std::collections::HashMap<String, &MailMessage> = state
        .messages
        .iter()
        .filter(|m| m.account_id == account_id)
        .map(|m| (m.id.clone(), m))
        .collect();

    let mut merged_messages: Vec<MailMessage> = Vec::with_capacity(remote_messages.len());
    let mut seen_ids: std::collections::HashSet<String> = std::collections::HashSet::new();

    for mut remote in remote_messages {
        seen_ids.insert(remote.id.clone());

        if let Some(local) = local_by_id.get(&remote.id) {
            // Preserve local flag/folder edits
            remote.is_read = local.is_read;
            remote.is_starred = local.is_starred;
            remote.folder_id = local.folder_id.clone();
        }

        merged_messages.push(remote);
    }

    // Keep local-only messages not present in the remote fetch
    // (messages beyond the 50-message window, locally created drafts/sent, etc.)
    for local in state.messages.iter().filter(|m| m.account_id == account_id) {
        if !seen_ids.contains(&local.id) {
            merged_messages.push(local.clone());
        }
    }

    state.messages.retain(|m| m.account_id != account_id);
    state.messages.extend(merged_messages);

    // Threads: replace (threads are derived from messages, no local edits)
    state.threads.retain(|t| t.account_id != account_id);
    state.threads.extend(remote_threads);

    state.recalculate_counts();
    state.persist()
}

pub fn finish_sync(account_id: &str, timestamp: &str) -> Result<SyncStatus, BackendError> {
    let mut state = lock_state();

    let status = SyncStatus {
        account_id: account_id.into(),
        state: "idle".into(),
        message: "Sync completed successfully".into(),
        updated_at: timestamp.into(),
    };

    if let Some(account_state) = state
        .account_states
        .iter_mut()
        .find(|s| s.account.id == account_id)
    {
        account_state.account.last_synced_at = timestamp.into();
        account_state.account.status = AccountStatus::Connected;
    }

    state.sync_statuses.insert(account_id.to_string(), status.clone());
    state.persist()?;
    Ok(status)
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

    fn recalculate_counts(&mut self) {
        for folder in self.folders.iter_mut() {
            if matches!(folder.kind, MailFolderKind::Starred) {
                let (unread, total) = self
                    .messages
                    .iter()
                    .filter(|m| m.account_id == folder.account_id && m.is_starred)
                    .fold((0u32, 0u32), |(u, t), m| {
                        (u + if m.is_read { 0 } else { 1 }, t + 1)
                    });
                folder.unread_count = unread;
                folder.total_count = total;
            } else {
                let (unread, total) = self
                    .messages
                    .iter()
                    .filter(|m| m.account_id == folder.account_id && m.folder_id == folder.id)
                    .fold((0u32, 0u32), |(u, t), m| {
                        (u + if m.is_read { 0 } else { 1 }, t + 1)
                    });
                folder.unread_count = unread;
                folder.total_count = total;
            }
        }

        for account_state in self.account_states.iter_mut() {
            let total_unread: u32 = self
                .messages
                .iter()
                .filter(|m| m.account_id == account_state.account.id && !m.is_read)
                .count() as u32;
            account_state.account.unread_count = total_unread;
        }
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
        let start = std::time::Instant::now();
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
            .map(|_| {
                eprintln!(
                    "[store] persisted ({} accounts, {} folders, {} messages, {} drafts) in {:.1?}",
                    self.account_states.len(),
                    self.folders.len(),
                    self.messages.len(),
                    self.drafts.len(),
                    start.elapsed(),
                );
            })
            .map_err(|error| BackendError::internal(error.to_string()))
    }
}

fn state() -> &'static Mutex<MemoryState> {
    static STATE: OnceLock<Mutex<MemoryState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(initial_state()))
}

fn initial_state() -> MemoryState {
    eprintln!("[store] loading initial state...");
    let seeded_accounts = accounts::seeded_account_states();
    let has_persisted = persisted::has_accounts_file();
    let account_states = if has_persisted {
        let loaded = persisted::load_accounts();
        eprintln!("[store] loaded {} accounts from disk", loaded.len());
        loaded
    } else {
        eprintln!("[store] no persisted data, using {} seed accounts", seeded_accounts.len());
        seeded_accounts
    };

    let seeded_mailbox = persisted::PersistedMailbox {
        folders: mailbox::seeded_folders(),
        messages: mailbox::seeded_messages(),
        threads: mailbox::seeded_threads(),
    };
    let mailbox_state = if persisted::has_mailbox_file() {
        persisted::load_mailbox()
    } else {
        seeded_mailbox
    };

    let seeded_drafts = drafts::seeded_drafts();
    let drafts = if persisted::has_drafts_file() {
        persisted::load_drafts()
    } else {
        seeded_drafts
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
    state.recalculate_counts();
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
            imap_uid: None,
        },
    );
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

pub fn current_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    // Manual UTC breakdown — avoids pulling in chrono just for timestamps
    let days = secs / 86400;
    let time_of_day = secs % 86400;
    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;

    // Civil date from days since 1970-01-01 (Euclidean algorithm)
    let z = days as i64 + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    format!("{y:04}-{m:02}-{d:02}T{hours:02}:{minutes:02}:{seconds:02}.000Z")
}
