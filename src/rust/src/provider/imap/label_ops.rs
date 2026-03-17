use futures::TryStreamExt;

use crate::models::{MailLabel, MailMessage};
use crate::provider::SyncProvider;
use crate::protocol::BackendError;
use crate::storage::memory;

pub(super) async fn list_labels(account_id: &str) -> Result<Vec<MailLabel>, BackendError> {
    imap_list_labels(account_id).await
}

pub(super) async fn add_label(
    account_id: &str,
    message_id: &str,
    label: &str,
) -> Result<Option<MailMessage>, BackendError> {
    let mail = memory::store().mail();
    let Some(message) = mail.get_message(account_id, message_id)? else {
        return Ok(None);
    };
    let uid = message
        .imap_uid
        .ok_or_else(|| BackendError::validation("This message does not have a remote IMAP UID"))?;
    imap_store_server_label(account_id, &message.folder_id, uid, label, true).await?;
    mail.add_label(account_id, message_id, label)
}

pub(super) async fn remove_label(
    account_id: &str,
    message_id: &str,
    label: &str,
) -> Result<Option<MailMessage>, BackendError> {
    let mail = memory::store().mail();
    let Some(message) = mail.get_message(account_id, message_id)? else {
        return Ok(None);
    };
    let uid = message
        .imap_uid
        .ok_or_else(|| BackendError::validation("This message does not have a remote IMAP UID"))?;
    imap_store_server_label(account_id, &message.folder_id, uid, label, false).await?;
    mail.remove_label(account_id, message_id, label)
}

pub(super) async fn rename_label(
    provider: &super::ImapSmtpProvider,
    account_id: &str,
    label: &str,
    new_label: &str,
) -> Result<Vec<MailLabel>, BackendError> {
    if imap_account_uses_gmail_labels(account_id).await? {
        imap_rename_gmail_label(account_id, label, new_label).await?;
        provider.sync_account_cap(account_id).await?;
        return imap_list_labels(account_id).await;
    }

    imap_rename_keyword_label(account_id, label, new_label).await?;
    provider.sync_account_cap(account_id).await?;
    imap_list_labels(account_id).await
}

pub(super) async fn delete_label(
    provider: &super::ImapSmtpProvider,
    account_id: &str,
    label: &str,
) -> Result<Vec<MailLabel>, BackendError> {
    if imap_account_uses_gmail_labels(account_id).await? {
        imap_delete_gmail_label(account_id, label).await?;
        provider.sync_account_cap(account_id).await?;
        return imap_list_labels(account_id).await;
    }

    imap_delete_keyword_label(account_id, label).await?;
    provider.sync_account_cap(account_id).await?;
    imap_list_labels(account_id).await
}

pub(super) fn normalize_keyword_label(label: &str) -> Option<String> {
    let trimmed = label.trim();
    if trimmed.is_empty() || trimmed.starts_with('\\') {
        return None;
    }
    Some(trimmed.into())
}

pub(super) async fn imap_fetch_gmail_labels(
    session: &mut super::ImapAnySession,
    uid_set: &str,
) -> Result<std::collections::HashMap<u32, Vec<String>>, BackendError> {
    let request_id = session
        .run_command(format!("UID FETCH {uid_set} (UID X-GM-LABELS)"))
        .await
        .map_err(|error| {
            BackendError::internal(format!("IMAP Gmail label fetch failed to send: {error}"))
        })?;

    let mut labels_by_uid = std::collections::HashMap::new();

    loop {
        let response = session.read_response().await.map_err(|error| {
            BackendError::internal(format!("IMAP Gmail label fetch read failed: {error}"))
        })?;
        let Some(response) = response else {
            return Err(BackendError::internal(
                "IMAP Gmail label fetch connection lost",
            ));
        };

        match response.parsed() {
            imap_proto::Response::Fetch(_, attrs) => {
                let mut uid = None;
                let mut labels = Vec::new();
                for attr in attrs {
                    match attr {
                        imap_proto::types::AttributeValue::Uid(value) => uid = Some(*value),
                        imap_proto::types::AttributeValue::GmailLabels(values) => {
                            labels = values
                                .iter()
                                .filter_map(|value| normalize_gmail_label(value.as_ref()))
                                .collect();
                        }
                        _ => {}
                    }
                }
                if let Some(uid) = uid {
                    labels_by_uid.insert(uid, labels);
                }
            }
            imap_proto::Response::Done {
                tag,
                status,
                information,
                ..
            } if tag == &request_id => match status {
                imap_proto::Status::Ok => break,
                other => {
                    return Err(BackendError::internal(format!(
                        "IMAP Gmail label fetch failed: {other:?} {information:?}"
                    )));
                }
            },
            _ => {}
        }
    }

    Ok(labels_by_uid)
}

async fn imap_store_server_label(
    account_id: &str,
    folder_id: &str,
    uid: u32,
    label: &str,
    add: bool,
) -> Result<(), BackendError> {
    let mailbox_name = super::folder_ops::get_imap_folder_name(account_id, folder_id)
        .ok_or_else(|| BackendError::internal("IMAP folder name not found"))?;

    let mut session = super::client_ops::imap_connect_by_account(account_id).await?;
    let capabilities = session
        .capabilities()
        .await
        .map_err(|error| BackendError::internal(format!("IMAP CAPABILITY failed: {error}")))?;

    session
        .select(&mailbox_name)
        .await
        .map_err(|e| BackendError::internal(format!("IMAP SELECT failed: {e}")))?;

    let result = if capabilities.has_str("X-GM-EXT-1") {
        imap_store_gmail_label(&mut session, uid, label, add).await
    } else {
        imap_store_keyword_label(&mut session, uid, label, add).await
    };

    let _ = session.logout().await;
    result
}

async fn imap_apply_keyword_label_batch(
    account_id: &str,
    label: &str,
    new_label: Option<&str>,
) -> Result<(), BackendError> {
    let source_label = validate_imap_keyword_label_name(label)?;
    let target_label = match new_label {
        Some(label) => Some(validate_imap_keyword_label_name(label)?),
        None => None,
    };

    for folder in memory::store().mail().list_folders(account_id)? {
        let Some(mailbox_name) = folder.imap_name.clone() else {
            continue;
        };

        let mut session = super::client_ops::imap_connect_by_account(account_id).await?;
        session
            .select(&mailbox_name)
            .await
            .map_err(|error| BackendError::internal(format!("IMAP SELECT failed: {error}")))?;

        let search_query = format!("KEYWORD {source_label}");
        let uids = session.uid_search(search_query).await.map_err(|error| {
            BackendError::internal(format!("IMAP keyword search failed: {error}"))
        })?;
        if uids.is_empty() {
            let _ = session.logout().await;
            continue;
        }

        let uid_set = uids
            .into_iter()
            .map(|uid| uid.to_string())
            .collect::<Vec<_>>()
            .join(",");
        if let Some(ref next_label) = target_label {
            let add_query = format!("+FLAGS ({next_label})");
            session
                .uid_store(&uid_set, &add_query)
                .await
                .map_err(|error| {
                    BackendError::internal(format!("IMAP keyword relabel add failed: {error}"))
                })?
                .try_collect::<Vec<_>>()
                .await
                .map_err(|error| {
                    BackendError::internal(format!("IMAP keyword relabel add failed: {error}"))
                })?;
        }

        let remove_query = format!("-FLAGS ({source_label})");
        session
            .uid_store(&uid_set, &remove_query)
            .await
            .map_err(|error| {
                BackendError::internal(format!("IMAP keyword relabel remove failed: {error}"))
            })?
            .try_collect::<Vec<_>>()
            .await
            .map_err(|error| {
                BackendError::internal(format!("IMAP keyword relabel remove failed: {error}"))
            })?;
        let _ = session.logout().await;
    }

    Ok(())
}

async fn imap_rename_keyword_label(
    account_id: &str,
    label: &str,
    new_label: &str,
) -> Result<(), BackendError> {
    imap_apply_keyword_label_batch(account_id, label, Some(new_label)).await
}

async fn imap_delete_keyword_label(account_id: &str, label: &str) -> Result<(), BackendError> {
    imap_apply_keyword_label_batch(account_id, label, None).await
}

async fn imap_rename_gmail_label(
    account_id: &str,
    label: &str,
    new_label: &str,
) -> Result<(), BackendError> {
    let current_name = super::parse_ops::encode_imap_utf7(label.trim());
    let next_name = super::parse_ops::encode_imap_utf7(new_label.trim());
    let mut session = super::client_ops::imap_connect_by_account(account_id).await?;
    session
        .rename(&current_name, &next_name)
        .await
        .map_err(|error| {
            BackendError::internal(format!("IMAP Gmail label rename failed: {error}"))
        })?;
    let _ = session.logout().await;
    Ok(())
}

async fn imap_delete_gmail_label(account_id: &str, label: &str) -> Result<(), BackendError> {
    let mailbox_name = super::parse_ops::encode_imap_utf7(label.trim());
    let mut session = super::client_ops::imap_connect_by_account(account_id).await?;
    session.delete(&mailbox_name).await.map_err(|error| {
        BackendError::internal(format!("IMAP Gmail label delete failed: {error}"))
    })?;
    let _ = session.logout().await;
    Ok(())
}

async fn imap_list_labels(account_id: &str) -> Result<Vec<MailLabel>, BackendError> {
    let local_labels = memory::store().mail().list_labels(account_id)?;
    if !imap_account_uses_gmail_labels(account_id).await? {
        return Ok(local_labels);
    }

    let mut counts = local_labels
        .into_iter()
        .map(|label| (label.name.to_lowercase(), label))
        .collect::<std::collections::HashMap<_, _>>();

    let mut session = super::client_ops::imap_connect_by_account(account_id).await?;
    let remote_folders = session
        .list(None, Some("*"))
        .await
        .map_err(|error| BackendError::internal(format!("IMAP LIST labels failed: {error}")))?
        .try_collect::<Vec<_>>()
        .await
        .map_err(|error| BackendError::internal(format!("IMAP LIST labels failed: {error}")))?;
    let _ = session.logout().await;

    for remote_folder in remote_folders {
        let name = super::parse_ops::decode_imap_utf7(remote_folder.name());
        if is_gmail_system_label(&name) {
            continue;
        }
        let key = name.to_lowercase();
        counts.entry(key).or_insert(MailLabel { name, count: 0 });
    }

    let mut labels = counts.into_values().collect::<Vec<_>>();
    labels.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));
    Ok(labels)
}

async fn imap_account_uses_gmail_labels(account_id: &str) -> Result<bool, BackendError> {
    let mut session = super::client_ops::imap_connect_by_account(account_id).await?;
    let capabilities = session
        .capabilities()
        .await
        .map_err(|error| BackendError::internal(format!("IMAP CAPABILITY failed: {error}")))?;
    let _ = session.logout().await;
    Ok(capabilities.has_str("X-GM-EXT-1"))
}

fn is_gmail_system_label(label: &str) -> bool {
    let lower = label.trim().to_lowercase();
    lower.starts_with("[gmail]/")
        || lower.starts_with("[google mail]/")
        || matches!(
            lower.as_str(),
            "inbox"
                | "sent"
                | "sent mail"
                | "draft"
                | "drafts"
                | "trash"
                | "spam"
                | "junk"
                | "all mail"
                | "starred"
                | "important"
        )
}

fn normalize_gmail_label(label: &str) -> Option<String> {
    let decoded = super::parse_ops::decode_imap_utf7(label).trim().to_string();
    if decoded.is_empty() || decoded.starts_with('\\') || is_gmail_system_label(&decoded) {
        return None;
    }
    Some(decoded)
}

fn validate_imap_keyword_label_name(label: &str) -> Result<String, BackendError> {
    let trimmed = label.trim();
    if trimmed.is_empty() {
        return Err(BackendError::validation("Label name cannot be empty"));
    }
    if !trimmed.is_ascii() {
        return Err(BackendError::validation(
            "This IMAP server only supports ASCII keyword labels",
        ));
    }
    if trimmed.chars().any(|ch| {
        ch.is_whitespace() || matches!(ch, '(' | ')' | '{' | '}' | '%' | '*' | '"' | '\\' | ']')
    }) {
        return Err(BackendError::validation(
            "This IMAP server does not allow this label name",
        ));
    }
    Ok(trimmed.into())
}

async fn imap_store_gmail_label(
    session: &mut super::ImapAnySession,
    uid: u32,
    label: &str,
    add: bool,
) -> Result<(), BackendError> {
    let encoded_label = super::parse_ops::quote_imap_string(
        &super::parse_ops::encode_imap_utf7(label.trim()),
    );
    let operation = if add { "+X-GM-LABELS" } else { "-X-GM-LABELS" };
    session
        .run_command_and_check_ok(format!("UID STORE {uid} {operation} ({encoded_label})"))
        .await
        .map_err(|error| {
            BackendError::internal(format!("IMAP Gmail label store failed: {error}"))
        })?;
    Ok(())
}

async fn imap_store_keyword_label(
    session: &mut super::ImapAnySession,
    uid: u32,
    label: &str,
    add: bool,
) -> Result<(), BackendError> {
    let keyword = validate_imap_keyword_label_name(label)?;
    let query = if add {
        format!("+FLAGS ({keyword})")
    } else {
        format!("-FLAGS ({keyword})")
    };
    session
        .uid_store(uid.to_string(), &query)
        .await
        .map_err(|error| {
            BackendError::internal(format!("IMAP keyword label store failed: {error}"))
        })?
        .try_collect::<Vec<_>>()
        .await
        .map_err(|error| {
            BackendError::internal(format!("IMAP keyword label store failed: {error}"))
        })?;
    Ok(())
}
