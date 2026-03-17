use futures::TryStreamExt;

use crate::models::{DraftMessage, MailFolderKind, MailMessage, MailboxFolder};
use crate::provider::SyncProvider;
use crate::protocol::BackendError;
use crate::storage::memory;

pub(super) async fn create_folder(
    provider: &super::ImapSmtpProvider,
    account_id: &str,
    name: &str,
) -> Result<Vec<MailboxFolder>, BackendError> {
    let mail = memory::store().mail();
    let mailbox_name = super::parse_ops::encode_imap_utf7(name.trim());
    let mut session = super::client_ops::imap_connect_by_account(account_id).await?;
    session
        .create(&mailbox_name)
        .await
        .map_err(|e| BackendError::internal(format!("IMAP CREATE failed: {e}")))?;
    let _ = session.logout().await;

    provider.sync_account_cap(account_id).await?;
    mail.list_folders(account_id)
}

pub(super) async fn rename_folder(
    provider: &super::ImapSmtpProvider,
    account_id: &str,
    folder_id: &str,
    name: &str,
) -> Result<Vec<MailboxFolder>, BackendError> {
    let mail = memory::store().mail();
    let folder = mail.get_folder(account_id, folder_id)?;
    if !matches!(folder.kind, MailFolderKind::Custom) {
        return Err(BackendError::validation(
            "Only custom folders can be renamed",
        ));
    }

    let current_name = folder
        .imap_name
        .clone()
        .unwrap_or_else(|| super::parse_ops::encode_imap_utf7(&folder.name));
    let next_name = super::parse_ops::encode_imap_utf7(name.trim());
    let mut session = super::client_ops::imap_connect_by_account(account_id).await?;
    session
        .rename(&current_name, &next_name)
        .await
        .map_err(|e| BackendError::internal(format!("IMAP RENAME failed: {e}")))?;
    let _ = session.logout().await;

    provider.sync_account_cap(account_id).await?;
    mail.list_folders(account_id)
}

pub(super) async fn delete_folder(
    provider: &super::ImapSmtpProvider,
    account_id: &str,
    folder_id: &str,
) -> Result<Vec<MailboxFolder>, BackendError> {
    let mail = memory::store().mail();
    let folder = mail.get_folder(account_id, folder_id)?;
    if !matches!(folder.kind, MailFolderKind::Custom) {
        return Err(BackendError::validation(
            "Only custom folders can be deleted",
        ));
    }
    if folder.total_count > 0 {
        return Err(BackendError::validation(
            "Only empty folders can be deleted",
        ));
    }

    let mailbox_name = folder
        .imap_name
        .clone()
        .unwrap_or_else(|| super::parse_ops::encode_imap_utf7(&folder.name));
    let mut session = super::client_ops::imap_connect_by_account(account_id).await?;
    session
        .delete(&mailbox_name)
        .await
        .map_err(|e| BackendError::internal(format!("IMAP DELETE failed: {e}")))?;
    let _ = session.logout().await;

    provider.sync_account_cap(account_id).await?;
    mail.list_folders(account_id)
}

pub(super) fn get_imap_folder_name(account_id: &str, folder_id: &str) -> Option<String> {
    memory::store().mail().list_folders(account_id).ok().and_then(|folders| {
        folders
            .into_iter()
            .find(|f| f.id == folder_id)
            .and_then(|f| f.imap_name)
    })
}

pub(super) fn get_drafts_mailbox_name(account_id: &str) -> Result<String, BackendError> {
    let folder = memory::store()
        .mail()
        .list_folders(account_id)?
        .into_iter()
        .find(|folder| matches!(folder.kind, MailFolderKind::Drafts))
        .ok_or_else(|| BackendError::not_found("Drafts folder not found"))?;
    Ok(folder
        .imap_name
        .unwrap_or_else(|| super::parse_ops::encode_imap_utf7(&folder.name)))
}

pub(super) fn find_matching_remote_draft(
    account_id: &str,
    draft: &DraftMessage,
) -> Result<Option<MailMessage>, BackendError> {
    let mail = memory::store().mail();
    let drafts_folder = mail
        .list_folders(account_id)?
        .into_iter()
        .find(|folder| matches!(folder.kind, MailFolderKind::Drafts))
        .ok_or_else(|| BackendError::not_found("Drafts folder not found"))?;
    let mut messages = mail.list_messages(account_id, &drafts_folder.id)?;
    let target_to = normalize_recipient_list(&split_recipients_for_match(&draft.to));
    let target_cc = normalize_recipient_list(&split_recipients_for_match(&draft.cc));
    messages.sort_by(|left, right| right.received_at.cmp(&left.received_at));
    Ok(messages.into_iter().find(|message| {
        message.imap_uid.is_some()
            && message.subject == draft.subject
            && message.body == draft.body
            && normalize_recipient_list(&message.to) == target_to
            && normalize_recipient_list(&message.cc) == target_cc
    }))
}

pub(super) async fn imap_delete_message_by_uid(
    account_id: &str,
    folder_id: &str,
    uid: u32,
) -> Result<(), BackendError> {
    let mailbox_name = get_imap_folder_name(account_id, folder_id)
        .ok_or_else(|| BackendError::not_found("IMAP folder not found"))?;
    let mut session = super::client_ops::imap_connect_by_account(account_id).await?;
    session
        .select(&mailbox_name)
        .await
        .map_err(|error| BackendError::internal(format!("IMAP SELECT failed: {error}")))?;
    let uid_str = uid.to_string();
    session
        .uid_store(&uid_str, "+FLAGS (\\Deleted)")
        .await
        .map_err(|error| BackendError::internal(format!("IMAP STORE \\Deleted failed: {error}")))?
        .try_collect::<Vec<_>>()
        .await
        .map_err(|error| BackendError::internal(format!("IMAP STORE \\Deleted failed: {error}")))?;
    session
        .expunge()
        .await
        .map_err(|error| BackendError::internal(format!("IMAP EXPUNGE failed: {error}")))?
        .try_collect::<Vec<_>>()
        .await
        .map_err(|error| BackendError::internal(format!("IMAP EXPUNGE failed: {error}")))?;
    let _ = session.logout().await;
    Ok(())
}

pub(super) async fn imap_move_message(
    account_id: &str,
    src_folder_id: &str,
    dest_folder_id: &str,
    uid: u32,
) -> Result<(), BackendError> {
    let src_name = get_imap_folder_name(account_id, src_folder_id)
        .ok_or_else(|| BackendError::internal("Source IMAP folder name not found"))?;
    let dest_name = get_imap_folder_name(account_id, dest_folder_id)
        .ok_or_else(|| BackendError::internal("Destination IMAP folder name not found"))?;

    let mut session = super::client_ops::imap_connect_by_account(account_id).await?;
    session
        .select(&src_name)
        .await
        .map_err(|e| BackendError::internal(format!("IMAP SELECT failed: {e}")))?;

    let uid_str = uid.to_string();
    session
        .uid_copy(&uid_str, &dest_name)
        .await
        .map_err(|e| BackendError::internal(format!("IMAP COPY failed: {e}")))?;

    session
        .uid_store(&uid_str, "+FLAGS (\\Deleted)")
        .await
        .map_err(|e| BackendError::internal(format!("IMAP STORE \\Deleted failed: {e}")))?
        .try_collect::<Vec<_>>()
        .await
        .map_err(|e| BackendError::internal(format!("IMAP STORE \\Deleted failed: {e}")))?;

    session
        .expunge()
        .await
        .map_err(|e| BackendError::internal(format!("IMAP EXPUNGE failed: {e}")))?
        .try_collect::<Vec<_>>()
        .await
        .map_err(|e| BackendError::internal(format!("IMAP EXPUNGE failed: {e}")))?;

    let _ = session.logout().await;
    Ok(())
}

fn normalize_recipient_list(value: &[String]) -> Vec<String> {
    value
        .iter()
        .map(|recipient| recipient.trim().to_lowercase())
        .filter(|recipient| !recipient.is_empty())
        .collect()
}

fn split_recipients_for_match(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(|recipient| recipient.trim().to_string())
        .filter(|recipient| !recipient.is_empty())
        .collect()
}
