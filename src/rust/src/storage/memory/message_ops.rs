use std::collections::HashMap;

use crate::models::{
    AttachmentContent, AttachmentMeta, DraftMessage, MailLabel, MailMessage, MailThread,
    MailboxBundle, MailFolderKind,
};
use crate::protocol::BackendError;
use crate::storage::{memory, sync};

pub(crate) fn get_local_attachment_content(
    account_id: &str,
    message_id: &str,
    attachment_id: &str,
) -> Result<Option<AttachmentContent>, BackendError> {
    let state = memory::lock_state();

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

pub(crate) fn search_messages(account_id: &str, query: &str) -> Result<Vec<MailMessage>, BackendError> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    let q = trimmed.to_lowercase();
    let state = memory::lock_state();
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

pub(crate) fn list_labels(account_id: &str) -> Result<Vec<MailLabel>, BackendError> {
    let state = memory::lock_state();
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

pub(crate) fn add_label(
    account_id: &str,
    message_id: &str,
    label: &str,
) -> Result<Option<MailMessage>, BackendError> {
    let normalized = normalize_label_name(label)?;
    let mut state = memory::lock_state();
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

pub(crate) fn remove_label(
    account_id: &str,
    message_id: &str,
    label: &str,
) -> Result<Option<MailMessage>, BackendError> {
    let normalized = normalize_label_name(label)?;
    let mut state = memory::lock_state();
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

pub(crate) fn get_message(
    account_id: &str,
    message_id: &str,
) -> Result<Option<MailMessage>, BackendError> {
    Ok(memory::lock_state()
        .messages
        .iter()
        .find(|message| message.account_id == account_id && message.id == message_id)
        .cloned())
}

pub(crate) fn save_draft(draft: DraftMessage) -> Result<DraftMessage, BackendError> {
    let mut state = memory::lock_state();

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

pub(crate) fn toggle_star(
    account_id: &str,
    message_id: &str,
) -> Result<Option<MailMessage>, BackendError> {
    let mut state = memory::lock_state();
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

pub(crate) fn toggle_read(
    account_id: &str,
    message_id: &str,
) -> Result<Option<MailMessage>, BackendError> {
    let mut state = memory::lock_state();
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

pub(crate) fn delete_message(account_id: &str, message_id: &str) -> Result<(), BackendError> {
    let mut state = memory::lock_state();
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

pub(crate) fn archive_message(
    account_id: &str,
    message_id: &str,
) -> Result<Option<MailMessage>, BackendError> {
    let mut state = memory::lock_state();
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

pub(crate) fn restore_message(
    account_id: &str,
    message_id: &str,
) -> Result<Option<MailMessage>, BackendError> {
    let mut state = memory::lock_state();
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

pub(crate) fn move_message(
    account_id: &str,
    message_id: &str,
    folder_id: &str,
) -> Result<Option<MailMessage>, BackendError> {
    let mut state = memory::lock_state();

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

pub(crate) fn mark_all_read(account_id: &str, folder_id: &str) -> Result<(), BackendError> {
    let mut state = memory::lock_state();
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

pub(crate) fn get_mailbox_bundle(account_id: &str) -> Result<MailboxBundle, BackendError> {
    let state = memory::lock_state();

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
            .unwrap_or_else(|| sync::initial_sync_status(account_id, &memory::current_timestamp())),
    })
}

pub(crate) fn get_existing_bodies(
    account_id: &str,
) -> HashMap<u32, (String, String, Vec<AttachmentMeta>, String)> {
    let state = memory::lock_state();
    let mut map = HashMap::new();
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

pub(crate) fn get_existing_bodies_for_folder(
    account_id: &str,
    folder_id: &str,
) -> HashMap<u32, (String, String, Vec<AttachmentMeta>, String)> {
    let state = memory::lock_state();
    let mut map = HashMap::new();
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

pub(crate) fn get_max_imap_uid_for_folder(account_id: &str, folder_id: &str) -> Option<u32> {
    let state = memory::lock_state();
    state
        .messages
        .iter()
        .filter(|m| m.account_id == account_id && m.folder_id == folder_id)
        .filter_map(|m| m.imap_uid)
        .max()
}

pub(crate) fn get_imap_uids_for_folder(account_id: &str, folder_id: &str) -> Vec<u32> {
    let state = memory::lock_state();
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

pub(crate) fn remove_messages_by_imap_uids(
    account_id: &str,
    folder_id: &str,
    uids: &[u32],
) -> Result<(), BackendError> {
    if uids.is_empty() {
        return Ok(());
    }

    let uid_set: std::collections::HashSet<u32> = uids.iter().copied().collect();
    let mut state = memory::lock_state();
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

pub(crate) fn get_existing_message_ids(account_id: &str) -> std::collections::HashSet<String> {
    let state = memory::lock_state();
    state
        .messages
        .iter()
        .filter(|m| m.account_id == account_id)
        .map(|m| m.id.clone())
        .collect()
}

pub(crate) fn record_sent_message(draft: DraftMessage) -> Result<(String, String), BackendError> {
    let mut state = memory::lock_state();
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
    let timestamp = memory::current_timestamp();
    let account = state
        .account_states
        .iter()
        .find(|account_state| account_state.account.id == draft.account_id)
        .map(|s| s.account.clone())
        .ok_or_else(|| BackendError::not_found("Account not found"))?;
    let identity = crate::storage::memory::account_ops::resolve_identity(
        &account,
        draft.selected_identity_id.as_deref(),
    );

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
            .map(|a| AttachmentMeta {
                id: a.file_name.clone(),
                file_name: a.file_name.clone(),
                mime_type: a.mime_type.clone(),
                size_bytes: a.data_base64.len() as u64 * 3 / 4,
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

pub(crate) fn sync_draft_mailbox_message(state: &mut memory::MemoryState, draft: &DraftMessage) {
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
            crate::storage::memory::account_ops::resolve_identity(
                &account_state.account,
                draft.selected_identity_id.as_deref(),
            )
        })
        .unwrap_or_else(|| crate::storage::memory::account_ops::default_identity("draft", "Draft", "draft@local"));
    if let Some(message) = state.messages.iter_mut().find(|message| message.id == draft.id) {
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
        message.received_at = memory::current_timestamp();
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
            sent_at: memory::current_timestamp(),
            received_at: memory::current_timestamp(),
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
