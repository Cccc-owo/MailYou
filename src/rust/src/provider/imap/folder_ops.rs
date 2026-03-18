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
    if let Err(error) = session.subscribe(&mailbox_name).await {
        eprintln!("[imap] SUBSCRIBE failed after create for {mailbox_name}: {error}");
    }
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
    if let Err(error) = session.unsubscribe(&current_name).await {
        eprintln!("[imap] UNSUBSCRIBE failed after rename for {current_name}: {error}");
    }
    if let Err(error) = session.subscribe(&next_name).await {
        eprintln!("[imap] SUBSCRIBE failed after rename for {next_name}: {error}");
    }
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
    if let Err(error) = session.unsubscribe(&mailbox_name).await {
        eprintln!("[imap] UNSUBSCRIBE failed after delete for {mailbox_name}: {error}");
    }
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
    imap_delete_messages_by_uid(account_id, folder_id, &[uid]).await
}

pub(super) async fn imap_delete_messages_by_uid(
    account_id: &str,
    folder_id: &str,
    uids: &[u32],
) -> Result<(), BackendError> {
    if uids.is_empty() {
        return Ok(());
    }

    let mailbox_name = get_imap_folder_name(account_id, folder_id)
        .ok_or_else(|| BackendError::not_found("IMAP folder not found"))?;
    let mut session = super::client_ops::imap_connect_by_account(account_id).await?;
    session
        .select(&mailbox_name)
        .await
        .map_err(|error| BackendError::internal(format!("IMAP SELECT failed: {error}")))?;
    let capabilities = session
        .capabilities()
        .await
        .map_err(|error| BackendError::internal(format!("IMAP CAPABILITY failed: {error}")))?;
    let uid_str = uids
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(",");
    session
        .uid_store(&uid_str, "+FLAGS (\\Deleted)")
        .await
        .map_err(|error| BackendError::internal(format!("IMAP STORE \\Deleted failed: {error}")))?
        .try_collect::<Vec<_>>()
        .await
        .map_err(|error| BackendError::internal(format!("IMAP STORE \\Deleted failed: {error}")))?;
    expunge_deleted_messages(&mut session, &uid_str, &capabilities).await?;
    let _ = session.logout().await;
    Ok(())
}

pub(super) async fn imap_move_message(
    account_id: &str,
    src_folder_id: &str,
    dest_folder_id: &str,
    uid: u32,
) -> Result<(), BackendError> {
    imap_move_messages(account_id, src_folder_id, dest_folder_id, &[uid]).await
}

pub(super) async fn imap_move_messages(
    account_id: &str,
    src_folder_id: &str,
    dest_folder_id: &str,
    uids: &[u32],
) -> Result<(), BackendError> {
    if uids.is_empty() {
        return Ok(());
    }

    let src_name = get_imap_folder_name(account_id, src_folder_id)
        .ok_or_else(|| BackendError::internal("Source IMAP folder name not found"))?;
    let dest_name = get_imap_folder_name(account_id, dest_folder_id)
        .ok_or_else(|| BackendError::internal("Destination IMAP folder name not found"))?;

    if src_name == dest_name {
        return Ok(());
    }

    let mut session = super::client_ops::imap_connect_by_account(account_id).await?;
    session
        .select(&src_name)
        .await
        .map_err(|e| BackendError::internal(format!("IMAP SELECT failed: {e}")))?;
    let capabilities = session
        .capabilities()
        .await
        .map_err(|error| BackendError::internal(format!("IMAP CAPABILITY failed: {error}")))?;

    let uid_str = uids
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(",");

    if capabilities.has_str("MOVE") {
        session
            .uid_mv(&uid_str, &dest_name)
            .await
            .map_err(|error| BackendError::internal(format!("IMAP MOVE failed: {error}")))?;
    } else {
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

        expunge_deleted_messages(&mut session, &uid_str, &capabilities).await?;
    }

    let _ = session.logout().await;
    Ok(())
}

async fn expunge_deleted_messages<T>(
    session: &mut async_imap::Session<T>,
    uid_set: &str,
    capabilities: &async_imap::types::Capabilities,
) -> Result<(), BackendError>
where
    T: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + std::fmt::Debug + Send,
{
    if capabilities.has_str("UIDPLUS") {
        session
            .uid_expunge(uid_set)
            .await
            .map_err(|error| BackendError::internal(format!("IMAP UID EXPUNGE failed: {error}")))?
            .try_collect::<Vec<_>>()
            .await
            .map_err(|error| BackendError::internal(format!("IMAP UID EXPUNGE failed: {error}")))?;
        return Ok(());
    }

    session
        .expunge()
        .await
        .map_err(|error| BackendError::internal(format!("IMAP EXPUNGE failed: {error}")))?
        .try_collect::<Vec<_>>()
        .await
        .map_err(|error| BackendError::internal(format!("IMAP EXPUNGE failed: {error}")))?;
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
