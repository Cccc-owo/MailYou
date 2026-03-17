use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, OnceLock};

use crate::models::{
    AccountSetupDraft, AccountStatus, AttachmentContent, AttachmentMeta, Contact, ContactGroup,
    DraftMessage, MailAccount, MailFolderKind, MailIdentity, MailLabel, MailMessage, MailThread,
    MailboxBundle, MailboxFolder, StoredAccountState, SyncStatus,
};
use crate::protocol::{BackendError, StorageSecurityStatus};
use crate::storage::{accounts, mailbox, persisted, sync};

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
    let identities = normalize_identities(&account_id, &draft.identities, base_name, &draft.email);
    let account = MailAccount {
        id: account_id.clone(),
        name: base_name.to_string(),
        email: draft.email.clone(),
        provider: draft.provider.clone(),
        incoming_protocol: draft.incoming_protocol.clone(),
        auth_mode: draft.auth_mode.clone(),
        oauth_provider: draft.oauth_provider.clone(),
        oauth_source: draft.oauth_source.clone(),
        color: "#5B8DEF".into(),
        initials: if initials.is_empty() {
            "NA".into()
        } else {
            initials
        },
        unread_count: 0,
        status: AccountStatus::Connected,
        last_synced_at: last_synced_at.clone(),
        identities,
    };

    state.insert_account_state(StoredAccountState {
        account: account.clone(),
        config,
    });
    state
        .folders
        .splice(0..0, mailbox::default_folders_for_account(&account_id));
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

pub fn get_folder(account_id: &str, folder_id: &str) -> Result<MailboxFolder, BackendError> {
    lock_state()
        .folders
        .iter()
        .find(|folder| folder.account_id == account_id && folder.id == folder_id)
        .cloned()
        .ok_or_else(|| BackendError::not_found("Folder not found"))
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

pub fn get_draft(account_id: &str, draft_id: &str) -> Result<Option<DraftMessage>, BackendError> {
    Ok(lock_state()
        .drafts
        .iter()
        .find(|draft| draft.account_id == account_id && draft.id == draft_id)
        .cloned())
}

pub fn remove_draft(account_id: &str, draft_id: &str) -> Result<(), BackendError> {
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

pub fn get_local_attachment_content(
    account_id: &str,
    message_id: &str,
    attachment_id: &str,
) -> Result<Option<AttachmentContent>, BackendError> {
    let state = lock_state();

    if let Some(draft) = state
        .drafts
        .iter()
        .find(|draft| draft.account_id == account_id && draft.id == message_id)
    {
        let attachment = draft
            .attachments
            .iter()
            .find(|attachment| attachment.file_name == attachment_id)
            .ok_or_else(|| BackendError::not_found("Attachment not found"))?;
        return Ok(Some(AttachmentContent {
            file_name: attachment.file_name.clone(),
            mime_type: attachment.mime_type.clone(),
            data_base64: attachment.data_base64.clone(),
        }));
    }

    Ok(None)
}

fn strip_html_tags(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut inside_tag = false;

    for ch in input.chars() {
        match ch {
            '<' => inside_tag = true,
            '>' => {
                inside_tag = false;
                output.push(' ');
            }
            _ if !inside_tag => output.push(ch),
            _ => {}
        }
    }

    output
}

pub fn search_messages(account_id: &str, query: &str) -> Result<Vec<MailMessage>, BackendError> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    let q = trimmed.to_lowercase();
    let state = lock_state();
    let mut messages: Vec<MailMessage> = state
        .messages
        .iter()
        .filter(|message| message.account_id == account_id)
        .filter(|message| {
            let recipients = message.to.join(" ");
            let cc = message.cc.join(" ");
            let labels = message.labels.join(" ");
            let haystack = format!(
                "{} {} {} {} {} {} {} {}",
                message.subject,
                message.preview,
                message.body,
                message.from,
                message.from_email,
                recipients,
                cc,
                labels,
            )
            .to_lowercase();

            if haystack.contains(&q) {
                return true;
            }

            strip_html_tags(&message.body).to_lowercase().contains(&q)
        })
        .cloned()
        .collect();

    messages.sort_by(|left, right| right.received_at.cmp(&left.received_at));
    Ok(messages)
}

fn normalize_label_name(label: &str) -> Result<String, BackendError> {
    let trimmed = label.trim();
    if trimmed.is_empty() {
        return Err(BackendError::validation("Label name cannot be empty"));
    }
    Ok(trimmed.into())
}

fn sort_message_labels(labels: &mut Vec<String>) {
    labels.sort_by_key(|label| label.to_lowercase());
    labels.dedup_by(|left, right| left.eq_ignore_ascii_case(right));
}

pub fn list_labels(account_id: &str) -> Result<Vec<MailLabel>, BackendError> {
    let state = lock_state();
    let mut counts: HashMap<String, (String, u32)> = HashMap::new();

    for message in state
        .messages
        .iter()
        .filter(|message| message.account_id == account_id)
    {
        for label in &message.labels {
            let key = label.to_lowercase();
            let entry = counts.entry(key).or_insert_with(|| (label.clone(), 0));
            entry.1 += 1;
        }
    }

    let mut labels: Vec<MailLabel> = counts
        .into_values()
        .map(|(name, count)| MailLabel { name, count })
        .collect();
    labels.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));
    Ok(labels)
}

pub fn add_label(
    account_id: &str,
    message_id: &str,
    label: &str,
) -> Result<Option<MailMessage>, BackendError> {
    let normalized = normalize_label_name(label)?;
    let mut state = lock_state();
    let updated = {
        let Some(message) = state
            .messages
            .iter_mut()
            .find(|message| message.account_id == account_id && message.id == message_id)
        else {
            return Ok(None);
        };

        if !message
            .labels
            .iter()
            .any(|existing| existing.eq_ignore_ascii_case(&normalized))
        {
            message.labels.push(normalized);
            sort_message_labels(&mut message.labels);
        }
        message.clone()
    };
    state.persist()?;
    Ok(Some(updated))
}

pub fn remove_label(
    account_id: &str,
    message_id: &str,
    label: &str,
) -> Result<Option<MailMessage>, BackendError> {
    let normalized = normalize_label_name(label)?;
    let mut state = lock_state();
    let updated = {
        let Some(message) = state
            .messages
            .iter_mut()
            .find(|message| message.account_id == account_id && message.id == message_id)
        else {
            return Ok(None);
        };

        message
            .labels
            .retain(|existing| !existing.eq_ignore_ascii_case(&normalized));
        message.clone()
    };
    state.persist()?;
    Ok(Some(updated))
}

pub fn get_message(
    account_id: &str,
    message_id: &str,
) -> Result<Option<MailMessage>, BackendError> {
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
        sync_draft_mailbox_message(&mut state, &updated);
        state.recalculate_counts();
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

pub fn toggle_star(
    account_id: &str,
    message_id: &str,
) -> Result<Option<MailMessage>, BackendError> {
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

pub fn toggle_read(
    account_id: &str,
    message_id: &str,
) -> Result<Option<MailMessage>, BackendError> {
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
        .find(|folder| {
            folder.account_id == account_id && matches!(folder.kind, MailFolderKind::Trash)
        })
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
    message.previous_folder_id = None;
    message.is_read = true;
    state.recalculate_counts();
    state.persist()?;
    Ok(())
}

pub fn delete_account(account_id: &str) -> Result<(), BackendError> {
    let mut state = lock_state();

    if !state
        .account_states
        .iter()
        .any(|s| s.account.id == account_id)
    {
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

pub fn archive_message(
    account_id: &str,
    message_id: &str,
) -> Result<Option<MailMessage>, BackendError> {
    let mut state = lock_state();
    let archive_folder_id = state
        .folders
        .iter()
        .find(|folder| {
            folder.account_id == account_id && matches!(folder.kind, MailFolderKind::Archive)
        })
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
    message.previous_folder_id = None;
    let updated = message.clone();
    state.recalculate_counts();
    state.persist()?;
    Ok(Some(updated))
}

pub fn restore_message(
    account_id: &str,
    message_id: &str,
) -> Result<Option<MailMessage>, BackendError> {
    let mut state = lock_state();
    let inbox_folder_id = state
        .folders
        .iter()
        .find(|folder| {
            folder.account_id == account_id && matches!(folder.kind, MailFolderKind::Inbox)
        })
        .map(|folder| folder.id.clone())
        .ok_or_else(|| BackendError::internal("Inbox folder is missing"))?;

    let restore_folder_id = {
        let previous_folder_id = state
            .messages
            .iter()
            .find(|message| message.account_id == account_id && message.id == message_id)
            .and_then(|message| message.previous_folder_id.clone());

        if let Some(folder_id) = previous_folder_id {
            if state
                .folders
                .iter()
                .any(|folder| folder.account_id == account_id && folder.id == folder_id)
            {
                folder_id
            } else {
                inbox_folder_id.clone()
            }
        } else {
            inbox_folder_id.clone()
        }
    };

    let Some(message) = state
        .messages
        .iter_mut()
        .find(|message| message.account_id == account_id && message.id == message_id)
    else {
        return Ok(None);
    };

    message.folder_id = restore_folder_id;
    message.previous_folder_id = None;
    let updated = message.clone();
    state.recalculate_counts();
    state.persist()?;
    Ok(Some(updated))
}

pub fn move_message(
    account_id: &str,
    message_id: &str,
    folder_id: &str,
) -> Result<Option<MailMessage>, BackendError> {
    let mut state = lock_state();

    let target_folder = state
        .folders
        .iter()
        .find(|folder| folder.account_id == account_id && folder.id == folder_id)
        .cloned();

    let Some(target_folder) = target_folder else {
        return Err(BackendError::not_found("Target folder not found"));
    };

    let Some(message) = state
        .messages
        .iter_mut()
        .find(|message| message.account_id == account_id && message.id == message_id)
    else {
        return Ok(None);
    };

    let current_folder_id = message.folder_id.clone();
    let moving_to_junk = matches!(target_folder.kind, MailFolderKind::Junk);
    let moving_out_of_junk = message.previous_folder_id.is_some() && !moving_to_junk;

    if moving_to_junk {
        message.previous_folder_id = Some(current_folder_id);
    } else if moving_out_of_junk {
        message.previous_folder_id = None;
    }

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

    let is_starred = folder
        .map(|f| matches!(f.kind, MailFolderKind::Starred))
        .unwrap_or(false);

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

/// Return cached (body, preview, attachments, received_at) keyed by IMAP UID for all messages that
/// belong to the given account and already have a downloaded body.
/// Used by incremental sync to skip re-fetching bodies we already have.
pub fn get_existing_bodies(
    account_id: &str,
) -> std::collections::HashMap<u32, (String, String, Vec<AttachmentMeta>, String)> {
    let state = lock_state();
    let mut map = std::collections::HashMap::new();
    for msg in state.messages.iter().filter(|m| m.account_id == account_id) {
        if let Some(uid) = msg.imap_uid {
            if !msg.body.is_empty() {
                map.insert(
                    uid,
                    (
                        msg.body.clone(),
                        msg.preview.clone(),
                        msg.attachments.clone(),
                        msg.received_at.clone(),
                    ),
                );
            }
        }
    }
    map
}

pub fn get_existing_bodies_for_folder(
    account_id: &str,
    folder_id: &str,
) -> std::collections::HashMap<u32, (String, String, Vec<AttachmentMeta>, String)> {
    let state = lock_state();
    let mut map = std::collections::HashMap::new();
    for msg in state
        .messages
        .iter()
        .filter(|m| m.account_id == account_id && m.folder_id == folder_id)
    {
        if let Some(uid) = msg.imap_uid {
            if !msg.body.is_empty() {
                map.insert(
                    uid,
                    (
                        msg.body.clone(),
                        msg.preview.clone(),
                        msg.attachments.clone(),
                        msg.received_at.clone(),
                    ),
                );
            }
        }
    }
    map
}

pub fn get_max_imap_uid_for_folder(account_id: &str, folder_id: &str) -> Option<u32> {
    let state = lock_state();
    state
        .messages
        .iter()
        .filter(|m| m.account_id == account_id && m.folder_id == folder_id)
        .filter_map(|m| m.imap_uid)
        .max()
}

pub fn get_imap_uids_for_folder(account_id: &str, folder_id: &str) -> Vec<u32> {
    let state = lock_state();
    let mut uids: Vec<u32> = state
        .messages
        .iter()
        .filter(|m| m.account_id == account_id && m.folder_id == folder_id)
        .filter_map(|m| m.imap_uid)
        .collect();
    uids.sort_unstable();
    uids.dedup();
    uids
}

pub fn remove_messages_by_imap_uids(
    account_id: &str,
    folder_id: &str,
    uids: &[u32],
) -> Result<(), BackendError> {
    if uids.is_empty() {
        return Ok(());
    }

    let uid_set: std::collections::HashSet<u32> = uids.iter().copied().collect();
    let mut state = lock_state();
    let removed_ids: std::collections::HashSet<String> = state
        .messages
        .iter()
        .filter(|message| message.account_id == account_id && message.folder_id == folder_id)
        .filter_map(|message| {
            message
                .imap_uid
                .filter(|uid| uid_set.contains(uid))
                .map(|_| message.id.clone())
        })
        .collect();

    if removed_ids.is_empty() {
        return Ok(());
    }

    state
        .messages
        .retain(|message| !removed_ids.contains(&message.id));
    state.threads.retain(|thread| {
        if thread.account_id != account_id {
            return true;
        }

        !thread
            .message_ids
            .iter()
            .any(|message_id| removed_ids.contains(message_id))
    });
    state.recalculate_counts();
    state.persist()
}

pub fn get_existing_message_ids(account_id: &str) -> std::collections::HashSet<String> {
    let state = lock_state();
    state
        .messages
        .iter()
        .filter(|m| m.account_id == account_id)
        .map(|m| m.id.clone())
        .collect()
}

pub fn get_account_state(account_id: &str) -> Option<StoredAccountState> {
    lock_state()
        .account_states
        .iter()
        .find(|s| s.account.id == account_id)
        .cloned()
}

pub fn get_account_config(account_id: &str) -> Result<AccountSetupDraft, BackendError> {
    let state = lock_state();
    let account_state = state
        .account_states
        .iter()
        .find(|s| s.account.id == account_id)
        .ok_or_else(|| BackendError::not_found("Account not found"))?;

    Ok(AccountSetupDraft {
        display_name: account_state.account.name.clone(),
        email: account_state.account.email.clone(),
        provider: account_state.account.provider.clone(),
        auth_mode: account_state.config.auth_mode.clone(),
        incoming_protocol: account_state.config.incoming_protocol.clone(),
        incoming_host: account_state.config.incoming_host.clone(),
        incoming_port: account_state.config.incoming_port,
        outgoing_host: account_state.config.outgoing_host.clone(),
        outgoing_port: account_state.config.outgoing_port,
        username: account_state.config.username.clone(),
        password: account_state.config.password.clone(),
        use_tls: account_state.config.use_tls,
        oauth_provider: account_state.config.oauth_provider.clone(),
        oauth_source: account_state.config.oauth_source.clone(),
        access_token: account_state.config.access_token.clone(),
        refresh_token: account_state.config.refresh_token.clone(),
        token_expires_at: account_state.config.token_expires_at.clone(),
        identities: account_state.account.identities.clone(),
    })
}

pub fn update_account(
    account_id: &str,
    draft: AccountSetupDraft,
) -> Result<MailAccount, BackendError> {
    let mut state = lock_state();
    let account_state = state
        .account_states
        .iter_mut()
        .find(|s| s.account.id == account_id)
        .ok_or_else(|| BackendError::not_found("Account not found"))?;

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

    account_state.account.name = base_name.to_string();
    account_state.account.email = draft.email.clone();
    account_state.account.provider = draft.provider.clone();
    account_state.account.incoming_protocol = draft.incoming_protocol.clone();
    account_state.account.auth_mode = draft.auth_mode.clone();
    account_state.account.oauth_provider = draft.oauth_provider.clone();
    account_state.account.oauth_source = draft.oauth_source.clone();
    account_state.account.initials = if initials.is_empty() {
        "NA".into()
    } else {
        initials
    };
    account_state.account.identities =
        normalize_identities(account_id, &draft.identities, base_name, &draft.email);
    account_state.config = accounts::config_from_draft(&draft);

    let updated = account_state.account.clone();
    state.persist()?;
    Ok(updated)
}

pub fn update_account_oauth_tokens(
    account_id: &str,
    access_token: &str,
    refresh_token: &str,
    expires_at: &str,
) -> Result<(), BackendError> {
    let mut state = lock_state();
    let account_state = state
        .account_states
        .iter_mut()
        .find(|s| s.account.id == account_id)
        .ok_or_else(|| BackendError::not_found("Account not found"))?;

    account_state.config.access_token = access_token.to_string();
    account_state.config.refresh_token = refresh_token.to_string();
    account_state.config.token_expires_at = expires_at.to_string();
    state.persist()
}

pub fn record_sent_message(draft: DraftMessage) -> Result<(String, String), BackendError> {
    let mut state = lock_state();
    state.drafts.retain(|item| item.id != draft.id);
    state.messages.retain(|message| message.id != draft.id);

    let sent_folder = state
        .folders
        .iter()
        .find(|folder| {
            folder.account_id == draft.account_id && matches!(folder.kind, MailFolderKind::Sent)
        })
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
    let identity = resolve_identity(&account, draft.selected_identity_id.as_deref());

    let message_id = format!("sent-{}", state.messages.len() + 1);
    let message = MailMessage {
        id: message_id.clone(),
        account_id: draft.account_id.clone(),
        folder_id: sent_folder,
        thread_id: thread_id.clone(),
        subject: draft.subject.clone(),
        preview: preview_for(&draft.body),
        body: draft.body.clone(),
        from: identity.name,
        from_email: identity.email,
        to: split_recipients(&draft.to),
        cc: split_recipients(&draft.cc),
        sent_at: timestamp.clone(),
        received_at: timestamp.clone(),
        is_read: true,
        is_starred: false,
        has_attachments: !draft.attachments.is_empty(),
        attachments: draft
            .attachments
            .iter()
            .map(|a| crate::models::AttachmentMeta {
                id: a.file_name.clone(),
                file_name: a.file_name.clone(),
                mime_type: a.mime_type.clone(),
                size_bytes: a.data_base64.len() as u64 * 3 / 4, // approximate decoded size
            })
            .collect(),
        labels: vec!["Sent".into()],
        previous_folder_id: None,
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
    Ok((message_id, timestamp))
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
            remote.previous_folder_id = local.previous_folder_id.clone();
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

pub fn merge_remote_folder(
    account_id: &str,
    folder: MailboxFolder,
    remote_messages: Vec<MailMessage>,
    remote_threads: Vec<MailThread>,
    prune_missing: bool,
) -> Result<(), BackendError> {
    let mut state = lock_state();
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

    state
        .sync_statuses
        .insert(account_id.to_string(), status.clone());
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
    contacts: Vec<Contact>,
    contact_groups: Vec<ContactGroup>,
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
            resolve_identity(
                &account_state.account,
                draft.selected_identity_id.as_deref(),
            )
        })
        .unwrap_or_else(|| default_identity("draft", "Draft", "draft@local"));
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

fn default_identity(account_id: &str, name: &str, email: &str) -> MailIdentity {
    MailIdentity {
        id: format!("identity-{account_id}-default"),
        name: name.into(),
        email: email.into(),
        reply_to: None,
        signature: None,
        is_default: true,
    }
}

fn normalize_identities(
    account_id: &str,
    identities: &[MailIdentity],
    fallback_name: &str,
    fallback_email: &str,
) -> Vec<MailIdentity> {
    let mut seen_ids = std::collections::HashSet::new();
    let mut normalized = identities
        .iter()
        .enumerate()
        .filter_map(|(index, identity)| {
            let email = identity.email.trim();
            if email.is_empty() {
                return None;
            }

            let name = identity.name.trim();
            let mut id = if identity.id.trim().is_empty() {
                format!("identity-{account_id}-{}", index + 1)
            } else {
                identity.id.trim().to_string()
            };
            if !seen_ids.insert(id.clone()) {
                id = format!("identity-{account_id}-{}", index + 1);
                seen_ids.insert(id.clone());
            }
            let reply_to = identity
                .reply_to
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned);
            let signature = identity
                .signature
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned);

            Some(MailIdentity {
                id,
                name: if name.is_empty() {
                    fallback_name.to_string()
                } else {
                    name.to_string()
                },
                email: email.to_string(),
                reply_to,
                signature,
                is_default: identity.is_default,
            })
        })
        .collect::<Vec<_>>();

    if normalized.is_empty() {
        return vec![default_identity(account_id, fallback_name, fallback_email)];
    }

    let default_index = normalized
        .iter()
        .position(|identity| identity.is_default)
        .unwrap_or(0);
    for (index, identity) in normalized.iter_mut().enumerate() {
        identity.is_default = index == default_index;
    }

    normalized
}

fn resolve_identity(account: &MailAccount, identity_id: Option<&str>) -> MailIdentity {
    if let Some(identity_id) = identity_id {
        if let Some(identity) = account
            .identities
            .iter()
            .find(|identity| identity.id == identity_id)
        {
            return identity.clone();
        }
    }

    account
        .identities
        .iter()
        .find(|identity| identity.is_default)
        .cloned()
        .or_else(|| account.identities.first().cloned())
        .unwrap_or_else(|| default_identity(&account.id, &account.name, &account.email))
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
    let state = lock_state();
    let mut out: Vec<Contact> = match group_id {
        Some(gid) => state
            .contacts
            .iter()
            .filter(|c| c.group_id.as_deref() == Some(gid))
            .cloned()
            .collect(),
        None => state.contacts.clone(),
    };
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(out)
}

pub fn create_contact(mut contact: Contact) -> Result<Contact, BackendError> {
    let mut state = lock_state();
    let now = current_timestamp();
    contact.id = format!("contact-{}", &uuid_short());
    contact.created_at = now.clone();
    contact.updated_at = now;
    state.contacts.push(contact.clone());
    state.persist()?;
    Ok(contact)
}

pub fn update_contact(contact_id: &str, mut contact: Contact) -> Result<Contact, BackendError> {
    let mut state = lock_state();
    let existing = state
        .contacts
        .iter_mut()
        .find(|c| c.id == contact_id)
        .ok_or_else(|| BackendError::not_found(format!("Contact '{contact_id}' not found")))?;
    contact.id = existing.id.clone();
    contact.created_at = existing.created_at.clone();
    contact.updated_at = current_timestamp();
    *existing = contact.clone();
    state.persist()?;
    Ok(contact)
}

pub fn delete_contact(contact_id: &str) -> Result<(), BackendError> {
    let mut state = lock_state();
    if state.contacts.iter().any(|c| c.id == contact_id) {
        let _ = persisted::delete_avatar(contact_id);
    }
    state.contacts.retain(|c| c.id != contact_id);
    state.persist()?;
    Ok(())
}

pub fn search_contacts(query: &str) -> Result<Vec<Contact>, BackendError> {
    let state = lock_state();
    let q = query.to_lowercase();
    let results: Vec<Contact> = state
        .contacts
        .iter()
        .filter(|c| {
            c.name.to_lowercase().contains(&q)
                || c.emails.iter().any(|e| e.to_lowercase().contains(&q))
        })
        .take(20)
        .cloned()
        .collect();
    Ok(results)
}

pub fn list_contact_groups() -> Result<Vec<ContactGroup>, BackendError> {
    Ok(lock_state().contact_groups.clone())
}

pub fn create_contact_group(name: String) -> Result<ContactGroup, BackendError> {
    let mut state = lock_state();
    let group = ContactGroup {
        id: format!("cg-{}", &uuid_short()),
        name,
    };
    state.contact_groups.push(group.clone());
    state.persist()?;
    Ok(group)
}

pub fn update_contact_group(group_id: &str, name: String) -> Result<ContactGroup, BackendError> {
    let mut state = lock_state();
    let group = state
        .contact_groups
        .iter_mut()
        .find(|g| g.id == group_id)
        .ok_or_else(|| BackendError::not_found(format!("Contact group '{group_id}' not found")))?;
    group.name = name;
    let updated = group.clone();
    state.persist()?;
    Ok(updated)
}

pub fn delete_contact_group(group_id: &str) -> Result<(), BackendError> {
    let mut state = lock_state();
    state.contact_groups.retain(|g| g.id != group_id);
    // Unlink contacts from the deleted group
    for contact in state.contacts.iter_mut() {
        if contact.group_id.as_deref() == Some(group_id) {
            contact.group_id = None;
        }
    }
    state.persist()?;
    Ok(())
}

pub fn upload_contact_avatar(
    contact_id: &str,
    data_base64: &str,
    mime_type: &str,
) -> Result<Contact, BackendError> {
    let decoded = base64_decode(data_base64)
        .map_err(|e| BackendError::validation(format!("Invalid base64: {e}")))?;
    persisted::save_avatar(contact_id, mime_type, &decoded)
        .map_err(|e| BackendError::internal(e.to_string()))?;

    let mut state = lock_state();
    let contact = state
        .contacts
        .iter_mut()
        .find(|c| c.id == contact_id)
        .ok_or_else(|| BackendError::not_found(format!("Contact '{contact_id}' not found")))?;
    contact.avatar_path = Some(contact_id.to_string());
    contact.updated_at = current_timestamp();
    let updated = contact.clone();
    state.persist()?;
    Ok(updated)
}

pub fn delete_contact_avatar(contact_id: &str) -> Result<Contact, BackendError> {
    let mut state = lock_state();
    let contact = state
        .contacts
        .iter_mut()
        .find(|c| c.id == contact_id)
        .ok_or_else(|| BackendError::not_found(format!("Contact '{contact_id}' not found")))?;
    persisted::delete_avatar(contact_id).map_err(|e| BackendError::internal(e.to_string()))?;

    contact.avatar_path = None;
    contact.updated_at = current_timestamp();
    let updated = contact.clone();
    state.persist()?;
    Ok(updated)
}

pub fn get_contact_avatar(contact_id: &str) -> Result<Option<AttachmentContent>, BackendError> {
    let Some((mime_type, payload)) =
        persisted::load_avatar(contact_id).map_err(|e| BackendError::internal(e.to_string()))?
    else {
        return Ok(None);
    };

    Ok(Some(AttachmentContent {
        file_name: format!("{contact_id}.webp"),
        mime_type,
        data_base64: base64_encode(&payload),
    }))
}

pub fn get_security_status() -> Result<StorageSecurityStatus, BackendError> {
    persisted::get_security_status().map_err(|e| BackendError::internal(e.to_string()))
}

pub fn unlock_storage(password: &str) -> Result<StorageSecurityStatus, BackendError> {
    persisted::unlock_storage(password).map_err(|e| BackendError::validation(e.to_string()))
}

pub fn set_master_password(
    current_password: Option<&str>,
    new_password: &str,
) -> Result<StorageSecurityStatus, BackendError> {
    if new_password.trim().len() < 8 {
        return Err(BackendError::validation(
            "Master password must be at least 8 characters long",
        ));
    }

    persisted::set_master_password(current_password, new_password)
        .map_err(|e| BackendError::validation(e.to_string()))
}

pub fn clear_master_password(
    current_password: &str,
) -> Result<StorageSecurityStatus, BackendError> {
    persisted::clear_master_password(current_password)
        .map_err(|e| BackendError::validation(e.to_string()))
}

pub fn get_storage_dir() -> Result<String, BackendError> {
    let dir = persisted::storage_dir_path().map_err(|e| BackendError::internal(e.to_string()))?;
    dir.to_str()
        .map(|s| s.to_string())
        .ok_or_else(|| BackendError::internal("Non-UTF-8 storage path"))
}

fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
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

fn base64_encode(input: &[u8]) -> String {
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

fn uuid_short() -> String {
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
