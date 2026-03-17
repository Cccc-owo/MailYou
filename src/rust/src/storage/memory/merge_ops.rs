use crate::models::{MailMessage, MailThread, MailboxFolder};
use crate::protocol::BackendError;
use crate::storage::memory;

pub(crate) fn merge_remote_mailbox(
    account_id: &str,
    folders: Vec<MailboxFolder>,
    remote_messages: Vec<MailMessage>,
    remote_threads: Vec<MailThread>,
) -> Result<(), BackendError> {
    let mut state = memory::lock_state();

    state.folders.retain(|f| f.account_id != account_id);
    state.folders.extend(folders);

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
            remote.is_read = local.is_read;
            remote.is_starred = local.is_starred;
            remote.folder_id = local.folder_id.clone();
            remote.previous_folder_id = local.previous_folder_id.clone();
        }

        merged_messages.push(remote);
    }

    for local in state.messages.iter().filter(|m| m.account_id == account_id) {
        if !seen_ids.contains(&local.id) {
            merged_messages.push(local.clone());
        }
    }

    state.messages.retain(|m| m.account_id != account_id);
    state.messages.extend(merged_messages);

    state.threads.retain(|t| t.account_id != account_id);
    state.threads.extend(remote_threads);

    state.recalculate_counts();
    state.persist()
}

pub(crate) fn merge_remote_folder(
    account_id: &str,
    folder: MailboxFolder,
    remote_messages: Vec<MailMessage>,
    remote_threads: Vec<MailThread>,
    prune_missing: bool,
) -> Result<(), BackendError> {
    let mut state = memory::lock_state();
    let folder_id = folder.id.clone();

    if let Some(existing_folder) = state
        .folders
        .iter_mut()
        .find(|existing| existing.account_id == account_id && existing.id == folder_id)
    {
        *existing_folder = folder;
    } else {
        state.folders.push(folder);
    }

    let local_by_id: std::collections::HashMap<String, &MailMessage> = state
        .messages
        .iter()
        .filter(|message| message.account_id == account_id)
        .map(|message| (message.id.clone(), message))
        .collect();
    let remote_ids: std::collections::HashSet<String> = remote_messages
        .iter()
        .map(|message| message.id.clone())
        .collect();

    let mut merged_remote = Vec::with_capacity(remote_messages.len());
    for mut remote in remote_messages {
        if let Some(local) = local_by_id.get(&remote.id) {
            remote.is_read = local.is_read;
            remote.is_starred = local.is_starred;
            remote.folder_id = local.folder_id.clone();
            remote.previous_folder_id = local.previous_folder_id.clone();
        }
        merged_remote.push(remote);
    }

    state.messages.retain(|message| {
        if message.account_id != account_id {
            return true;
        }

        if !remote_ids.contains(&message.id) {
            return true;
        }

        if message.folder_id != folder_id {
            return true;
        }

        false
    });

    if prune_missing {
        state.messages.retain(|message| {
            if message.account_id != account_id || message.folder_id != folder_id {
                return true;
            }

            remote_ids.contains(&message.id)
        });
    }

    state.messages.extend(merged_remote);

    let remote_thread_ids: std::collections::HashSet<String> = remote_threads
        .iter()
        .map(|thread| thread.id.clone())
        .collect();
    state.threads.retain(|thread| {
        !(thread.account_id == account_id && remote_thread_ids.contains(&thread.id))
    });
    state.threads.extend(remote_threads);

    state.recalculate_counts();
    state.persist()
}
