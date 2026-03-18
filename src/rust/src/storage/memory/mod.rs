use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, OnceLock};

mod account_ops;
mod contacts;
mod merge_ops;
mod message_ops;
mod security;

use crate::models::{
    AccountSetupDraft, AccountUnreadSnapshot, AttachmentContent, AttachmentMeta, Contact,
    ContactGroup, DraftMessage, MailAccount, MailFolderKind, MailLabel, MailMessage, MailThread,
    MailboxBundle, MailboxFolder, StoredAccountState, SyncStatus,
};
use crate::protocol::{BackendError, StorageSecurityStatus};
use crate::storage::persisted;

#[derive(Clone, Copy)]
pub struct MemoryStore;

#[allow(dead_code)]
#[derive(Clone, Copy)]
pub struct AccountStore;

#[allow(dead_code)]
#[derive(Clone, Copy)]
pub struct MailStore;

#[derive(Clone, Copy)]
pub struct ContactStore;

#[derive(Clone, Copy)]
pub struct SecurityStore;

pub fn store() -> MemoryStore {
    MemoryStore
}

impl MemoryStore {
    pub fn accounts(self) -> AccountStore {
        AccountStore
    }

    #[allow(dead_code)]
    pub fn mail(self) -> MailStore {
        MailStore
    }

    pub fn contacts(self) -> ContactStore {
        ContactStore
    }

    pub fn security(self) -> SecurityStore {
        SecurityStore
    }
}

#[allow(dead_code)]
impl AccountStore {
    pub fn list_accounts(self) -> Result<Vec<MailAccount>, BackendError> {
        Ok(lock_state().accounts())
    }

    pub fn create_account_without_test(
        self,
        draft: AccountSetupDraft,
    ) -> Result<MailAccount, BackendError> {
        account_ops::create_account_without_test(draft)
    }

    pub fn get_account_state(self, account_id: &str) -> Option<StoredAccountState> {
        account_ops::get_account_state(account_id)
    }

    pub fn get_account_config(self, account_id: &str) -> Result<AccountSetupDraft, BackendError> {
        account_ops::get_account_config(account_id)
    }

    pub fn update_account(
        self,
        account_id: &str,
        draft: AccountSetupDraft,
    ) -> Result<MailAccount, BackendError> {
        account_ops::update_account(account_id, draft)
    }

    pub fn update_account_oauth_tokens(
        self,
        account_id: &str,
        access_token: &str,
        refresh_token: &str,
        expires_at: &str,
    ) -> Result<(), BackendError> {
        account_ops::update_account_oauth_tokens(account_id, access_token, refresh_token, expires_at)
    }

    pub fn delete_account(self, account_id: &str) -> Result<(), BackendError> {
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

    pub fn finish_sync(self, account_id: &str, timestamp: &str) -> Result<SyncStatus, BackendError> {
        account_ops::finish_sync(account_id, timestamp)
    }
}

#[allow(dead_code)]
impl MailStore {
    pub fn list_folders(self, account_id: &str) -> Result<Vec<MailboxFolder>, BackendError> {
        Ok(lock_state()
            .folders
            .iter()
            .filter(|folder| folder.account_id == account_id)
            .cloned()
            .collect())
    }

    pub fn get_folder(self, account_id: &str, folder_id: &str) -> Result<MailboxFolder, BackendError> {
        lock_state()
            .folders
            .iter()
            .find(|folder| folder.account_id == account_id && folder.id == folder_id)
            .cloned()
            .ok_or_else(|| BackendError::not_found("Folder not found"))
    }

    pub fn list_messages(self, account_id: &str, folder_id: &str) -> Result<Vec<MailMessage>, BackendError> {
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

    pub fn get_message(self, account_id: &str, message_id: &str) -> Result<Option<MailMessage>, BackendError> {
        message_ops::get_message(account_id, message_id)
    }

    pub fn get_draft(self, account_id: &str, draft_id: &str) -> Result<Option<DraftMessage>, BackendError> {
        Ok(lock_state()
            .drafts
            .iter()
            .find(|draft| draft.account_id == account_id && draft.id == draft_id)
            .cloned())
    }

    pub fn save_draft(self, draft: DraftMessage) -> Result<DraftMessage, BackendError> {
        message_ops::save_draft(draft)
    }

    pub fn remove_draft(self, account_id: &str, draft_id: &str) -> Result<(), BackendError> {
        let mut state = lock_state();
        state
            .drafts
            .retain(|draft| !(draft.account_id == account_id && draft.id == draft_id));
        state
            .messages
            .retain(|message| !(message.account_id == account_id && message.id == draft_id));
        state.recalculate_counts();
        state.persist()
    }

    pub fn search_messages(self, account_id: &str, query: &str) -> Result<Vec<MailMessage>, BackendError> {
        message_ops::search_messages(account_id, query)
    }

    pub fn list_labels(self, account_id: &str) -> Result<Vec<MailLabel>, BackendError> {
        message_ops::list_labels(account_id)
    }

    pub fn add_label(
        self,
        account_id: &str,
        message_id: &str,
        label: &str,
    ) -> Result<Option<MailMessage>, BackendError> {
        message_ops::add_label(account_id, message_id, label)
    }

    pub fn remove_label(
        self,
        account_id: &str,
        message_id: &str,
        label: &str,
    ) -> Result<Option<MailMessage>, BackendError> {
        message_ops::remove_label(account_id, message_id, label)
    }

    pub fn get_attachment_content(
        self,
        account_id: &str,
        message_id: &str,
        attachment_id: &str,
    ) -> Result<Option<AttachmentContent>, BackendError> {
        message_ops::get_local_attachment_content(account_id, message_id, attachment_id)
    }

    pub fn toggle_star(
        self,
        account_id: &str,
        message_id: &str,
    ) -> Result<Option<MailMessage>, BackendError> {
        message_ops::toggle_star(account_id, message_id)
    }

    pub fn toggle_read(
        self,
        account_id: &str,
        message_id: &str,
    ) -> Result<Option<MailMessage>, BackendError> {
        message_ops::toggle_read(account_id, message_id)
    }

    pub fn batch_toggle_read(
        self,
        account_id: &str,
        message_ids: &[String],
        is_read: bool,
    ) -> Result<(), BackendError> {
        message_ops::batch_toggle_read(account_id, message_ids, is_read)
    }

    pub fn delete_message(self, account_id: &str, message_id: &str) -> Result<(), BackendError> {
        message_ops::delete_message(account_id, message_id)
    }

    pub fn batch_delete_messages(
        self,
        account_id: &str,
        message_ids: &[String],
    ) -> Result<(), BackendError> {
        message_ops::batch_delete_messages(account_id, message_ids)
    }

    pub fn archive_message(
        self,
        account_id: &str,
        message_id: &str,
    ) -> Result<Option<MailMessage>, BackendError> {
        message_ops::archive_message(account_id, message_id)
    }

    pub fn restore_message(
        self,
        account_id: &str,
        message_id: &str,
    ) -> Result<Option<MailMessage>, BackendError> {
        message_ops::restore_message(account_id, message_id)
    }

    pub fn move_message(
        self,
        account_id: &str,
        message_id: &str,
        folder_id: &str,
    ) -> Result<Option<MailMessage>, BackendError> {
        message_ops::move_message(account_id, message_id, folder_id)
    }

    pub fn batch_move_messages(
        self,
        account_id: &str,
        message_ids: &[String],
        folder_id: &str,
    ) -> Result<(), BackendError> {
        message_ops::batch_move_messages(account_id, message_ids, folder_id)
    }

    pub fn mark_all_read(self, account_id: &str, folder_id: &str) -> Result<(), BackendError> {
        message_ops::mark_all_read(account_id, folder_id)
    }

    pub fn get_mailbox_bundle(self, account_id: &str) -> Result<MailboxBundle, BackendError> {
        message_ops::get_mailbox_bundle(account_id)
    }

    pub fn get_account_unread_snapshot(
        self,
        account_id: &str,
    ) -> Result<AccountUnreadSnapshot, BackendError> {
        message_ops::get_account_unread_snapshot(account_id)
    }

    pub fn merge_remote_mailbox(
        self,
        account_id: &str,
        folders: Vec<MailboxFolder>,
        remote_messages: Vec<MailMessage>,
        remote_threads: Vec<MailThread>,
    ) -> Result<(), BackendError> {
        merge_ops::merge_remote_mailbox(account_id, folders, remote_messages, remote_threads)
    }

    pub fn merge_remote_folder(
        self,
        account_id: &str,
        folder: MailboxFolder,
        remote_messages: Vec<MailMessage>,
        remote_threads: Vec<MailThread>,
        prune_missing: bool,
    ) -> Result<(), BackendError> {
        merge_ops::merge_remote_folder(account_id, folder, remote_messages, remote_threads, prune_missing)
    }

    pub fn get_existing_bodies(
        self,
        account_id: &str,
    ) -> std::collections::HashMap<u32, (String, String, Vec<AttachmentMeta>, String)> {
        message_ops::get_existing_bodies(account_id)
    }

    pub fn get_existing_bodies_for_folder(
        self,
        account_id: &str,
        folder_id: &str,
    ) -> std::collections::HashMap<u32, (String, String, Vec<AttachmentMeta>, String)> {
        message_ops::get_existing_bodies_for_folder(account_id, folder_id)
    }

    pub fn get_max_imap_uid_for_folder(self, account_id: &str, folder_id: &str) -> Option<u32> {
        message_ops::get_max_imap_uid_for_folder(account_id, folder_id)
    }

    pub fn get_imap_uids_for_folder(self, account_id: &str, folder_id: &str) -> Vec<u32> {
        message_ops::get_imap_uids_for_folder(account_id, folder_id)
    }

    pub fn remove_messages_by_imap_uids(
        self,
        account_id: &str,
        folder_id: &str,
        uids: &[u32],
    ) -> Result<(), BackendError> {
        message_ops::remove_messages_by_imap_uids(account_id, folder_id, uids)
    }

    pub fn get_existing_message_ids(self, account_id: &str) -> std::collections::HashSet<String> {
        message_ops::get_existing_message_ids(account_id)
    }

    pub fn record_sent_message(self, draft: DraftMessage) -> Result<(String, String), BackendError> {
        message_ops::record_sent_message(draft)
    }
}

impl ContactStore {
    pub fn list_contacts(self, group_id: Option<&str>) -> Result<Vec<Contact>, BackendError> {
        list_contacts(group_id)
    }

    pub fn create_contact(self, contact: Contact) -> Result<Contact, BackendError> {
        create_contact(contact)
    }

    pub fn update_contact(self, contact_id: &str, contact: Contact) -> Result<Contact, BackendError> {
        update_contact(contact_id, contact)
    }

    pub fn delete_contact(self, contact_id: &str) -> Result<(), BackendError> {
        delete_contact(contact_id)
    }

    pub fn search_contacts(self, query: &str) -> Result<Vec<Contact>, BackendError> {
        search_contacts(query)
    }

    pub fn list_contact_groups(self) -> Result<Vec<ContactGroup>, BackendError> {
        list_contact_groups()
    }

    pub fn create_contact_group(self, name: String) -> Result<ContactGroup, BackendError> {
        create_contact_group(name)
    }

    pub fn update_contact_group(self, group_id: &str, name: String) -> Result<ContactGroup, BackendError> {
        update_contact_group(group_id, name)
    }

    pub fn delete_contact_group(self, group_id: &str) -> Result<(), BackendError> {
        delete_contact_group(group_id)
    }

    pub fn upload_contact_avatar(
        self,
        contact_id: &str,
        data_base64: &str,
        mime_type: &str,
    ) -> Result<Contact, BackendError> {
        upload_contact_avatar(contact_id, data_base64, mime_type)
    }

    pub fn delete_contact_avatar(self, contact_id: &str) -> Result<Contact, BackendError> {
        delete_contact_avatar(contact_id)
    }

    pub fn get_contact_avatar(
        self,
        contact_id: &str,
    ) -> Result<Option<AttachmentContent>, BackendError> {
        get_contact_avatar(contact_id)
    }
}

impl SecurityStore {
    pub fn get_security_status(self) -> Result<StorageSecurityStatus, BackendError> {
        get_security_status()
    }

    pub fn unlock_storage(self, password: &str) -> Result<StorageSecurityStatus, BackendError> {
        unlock_storage(password)
    }

    pub fn set_master_password(
        self,
        current_password: Option<&str>,
        new_password: &str,
    ) -> Result<StorageSecurityStatus, BackendError> {
        set_master_password(current_password, new_password)
    }

    pub fn clear_master_password(
        self,
        current_password: &str,
    ) -> Result<StorageSecurityStatus, BackendError> {
        clear_master_password(current_password)
    }

    pub fn get_storage_dir(self) -> Result<String, BackendError> {
        get_storage_dir()
    }
}

/// Lock the global state, recovering from a poisoned Mutex if necessary.
/// A poisoned Mutex means a previous thread panicked while holding the lock.
/// The data may be inconsistent, but it's better to continue than to panic
/// (which would leave the request without a response).
pub(crate) fn lock_state() -> MutexGuard<'static, MemoryState> {
    state().lock().unwrap_or_else(|poisoned| {
        eprintln!("[backend] WARNING: recovering from poisoned Mutex");
        poisoned.into_inner()
    })
}

#[derive(Clone)]
pub(crate) struct MemoryState {
    account_states: Vec<StoredAccountState>,
    folders: Vec<MailboxFolder>,
    messages: Vec<MailMessage>,
    threads: Vec<MailThread>,
    drafts: Vec<DraftMessage>,
    sync_statuses: HashMap<String, SyncStatus>,
    contacts: Vec<Contact>,
    contact_groups: Vec<ContactGroup>,
}

impl MemoryState {
    pub(crate) fn accounts(&self) -> Vec<MailAccount> {
        self.account_states
            .iter()
            .map(|state| state.account.clone())
            .collect()
    }

    pub(crate) fn insert_account_state(&mut self, account_state: StoredAccountState) {
        self.account_states.insert(0, account_state);
    }

    pub(crate) fn recalculate_counts(&mut self) {
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

    pub(crate) fn unique_account_id(&self, email: &str) -> String {
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

    pub(crate) fn persist(&self) -> Result<(), BackendError> {
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
            .and_then(|_| {
                persisted::save_contacts(&persisted::PersistedContacts {
                    contacts: self.contacts.clone(),
                    groups: self.contact_groups.clone(),
                })
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
    let has_persisted = persisted::has_accounts_file();
    let account_states = if has_persisted {
        let loaded = persisted::load_accounts();
        eprintln!("[store] loaded {} accounts from disk", loaded.len());
        loaded
    } else {
        eprintln!("[store] no persisted account data found");
        Vec::new()
    };

    let mailbox_state = if persisted::has_mailbox_file() {
        persisted::load_mailbox()
    } else {
        persisted::PersistedMailbox::default()
    };

    let drafts = if persisted::has_drafts_file() {
        persisted::load_drafts()
    } else {
        Vec::new()
    };

    let sync_statuses = {
        let loaded = persisted::load_sync_statuses();
        if loaded.is_empty() {
            HashMap::new()
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
        contacts: Vec::new(),
        contact_groups: Vec::new(),
    };

    // Load contacts
    if persisted::has_contacts_file() {
        let loaded = persisted::load_contacts();
        eprintln!(
            "[store] loaded {} contacts, {} groups from disk",
            loaded.contacts.len(),
            loaded.groups.len()
        );
        state.contacts = loaded.contacts;
        state.contact_groups = loaded.groups;
    }

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
        .find(|folder| {
            folder.account_id == draft.account_id && matches!(folder.kind, MailFolderKind::Drafts)
        })
        .map(|folder| folder.id.clone())
    else {
        return;
    };

    let preview = preview_for(&draft.body);
    let attachments = draft
        .attachments
        .iter()
        .map(|attachment| AttachmentMeta {
            id: attachment.file_name.clone(),
            file_name: attachment.file_name.clone(),
            mime_type: attachment.mime_type.clone(),
            size_bytes: attachment.data_base64.len() as u64 * 3 / 4,
        })
        .collect::<Vec<_>>();
    let identity = state
        .account_states
        .iter()
        .find(|account_state| account_state.account.id == draft.account_id)
        .map(|account_state| {
            account_ops::resolve_identity(
                &account_state.account,
                draft.selected_identity_id.as_deref(),
            )
        })
        .unwrap_or_else(|| account_ops::default_identity("draft", "Draft", "draft@local"));
    if let Some(message) = state
        .messages
        .iter_mut()
        .find(|message| message.id == draft.id)
    {
        message.folder_id = folder_id;
        message.subject = draft.subject.clone();
        message.preview = preview.clone();
        message.body = draft.body.clone();
        message.to = split_recipients(&draft.to);
        message.cc = split_recipients(&draft.cc);
        message.from = identity.name.clone();
        message.from_email = identity.email.clone();
        message.has_attachments = !attachments.is_empty();
        message.attachments = attachments;
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
            from: identity.name,
            from_email: identity.email,
            to: split_recipients(&draft.to),
            cc: split_recipients(&draft.cc),
            sent_at: current_timestamp(),
            received_at: current_timestamp(),
            is_read: true,
            is_starred: false,
            has_attachments: !attachments.is_empty(),
            attachments,
            labels: vec!["Draft".into()],
            previous_folder_id: None,
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

// ---------------------------------------------------------------------------
// Contacts
// ---------------------------------------------------------------------------

pub fn list_contacts(group_id: Option<&str>) -> Result<Vec<Contact>, BackendError> {
    contacts::list_contacts(group_id)
}

pub fn create_contact(contact: Contact) -> Result<Contact, BackendError> {
    contacts::create_contact(contact)
}

pub fn update_contact(contact_id: &str, contact: Contact) -> Result<Contact, BackendError> {
    contacts::update_contact(contact_id, contact)
}

pub fn delete_contact(contact_id: &str) -> Result<(), BackendError> {
    contacts::delete_contact(contact_id)
}

pub fn search_contacts(query: &str) -> Result<Vec<Contact>, BackendError> {
    contacts::search_contacts(query)
}

pub fn list_contact_groups() -> Result<Vec<ContactGroup>, BackendError> {
    contacts::list_contact_groups()
}

pub fn create_contact_group(name: String) -> Result<ContactGroup, BackendError> {
    contacts::create_contact_group(name)
}

pub fn update_contact_group(group_id: &str, name: String) -> Result<ContactGroup, BackendError> {
    contacts::update_contact_group(group_id, name)
}

pub fn delete_contact_group(group_id: &str) -> Result<(), BackendError> {
    contacts::delete_contact_group(group_id)
}

pub fn upload_contact_avatar(
    contact_id: &str,
    data_base64: &str,
    mime_type: &str,
) -> Result<Contact, BackendError> {
    contacts::upload_contact_avatar(contact_id, data_base64, mime_type)
}

pub fn delete_contact_avatar(contact_id: &str) -> Result<Contact, BackendError> {
    contacts::delete_contact_avatar(contact_id)
}

pub fn get_contact_avatar(contact_id: &str) -> Result<Option<AttachmentContent>, BackendError> {
    contacts::get_contact_avatar(contact_id)
}

pub fn get_security_status() -> Result<StorageSecurityStatus, BackendError> {
    security::get_security_status()
}

pub fn unlock_storage(password: &str) -> Result<StorageSecurityStatus, BackendError> {
    security::unlock_storage(password)
}

pub fn set_master_password(
    current_password: Option<&str>,
    new_password: &str,
) -> Result<StorageSecurityStatus, BackendError> {
    security::set_master_password(current_password, new_password)
}

pub fn clear_master_password(
    current_password: &str,
) -> Result<StorageSecurityStatus, BackendError> {
    security::clear_master_password(current_password)
}

pub fn get_storage_dir() -> Result<String, BackendError> {
    security::get_storage_dir()
}

pub(crate) fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    let cleaned: String = input.chars().filter(|c| !c.is_whitespace()).collect();
    let mut result = Vec::with_capacity(cleaned.len() * 3 / 4);

    let alphabet = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut buf: u32 = 0;
    let mut bits: u32 = 0;

    for ch in cleaned.bytes() {
        if ch == b'=' {
            break;
        }
        let val = alphabet
            .iter()
            .position(|&b| b == ch)
            .ok_or_else(|| format!("Invalid base64 char: {}", ch as char))?
            as u32;
        buf = (buf << 6) | val;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            result.push((buf >> bits) as u8);
            buf &= (1 << bits) - 1;
        }
    }
    Ok(result)
}

pub(crate) fn base64_encode(input: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = String::with_capacity(input.len().div_ceil(3) * 4);

    for chunk in input.chunks(3) {
        let b0 = chunk[0];
        let b1 = *chunk.get(1).unwrap_or(&0);
        let b2 = *chunk.get(2).unwrap_or(&0);
        let n = ((b0 as u32) << 16) | ((b1 as u32) << 8) | b2 as u32;

        output.push(ALPHABET[((n >> 18) & 0x3f) as usize] as char);
        output.push(ALPHABET[((n >> 12) & 0x3f) as usize] as char);
        output.push(if chunk.len() > 1 {
            ALPHABET[((n >> 6) & 0x3f) as usize] as char
        } else {
            '='
        });
        output.push(if chunk.len() > 2 {
            ALPHABET[(n & 0x3f) as usize] as char
        } else {
            '='
        });
    }

    output
}

pub(crate) fn uuid_short() -> String {
    // Simple unique-enough ID: take first 12 chars from a hex timestamp + counter
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{ts:x}{seq:x}")
}
