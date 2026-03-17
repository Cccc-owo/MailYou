use futures::TryStreamExt;

use crate::models::{AttachmentContent, DraftMessage, MailFolderKind, MailMessage};
use crate::provider::common::{
    finalize_smtp_send, get_attachment_content_from_storage, log_smtp_send_start,
    prepare_smtp_send,
};
use crate::provider::SyncProvider;
use crate::protocol::BackendError;
use crate::storage::memory;

pub(super) async fn get_draft(
    account_id: &str,
    draft_id: &str,
) -> Result<Option<DraftMessage>, BackendError> {
    let mail = memory::store().mail();
    if let Some(draft) = mail.get_draft(account_id, draft_id)? {
        return Ok(Some(draft));
    }

    let Some(message) = mail.get_message(account_id, draft_id)? else {
        return Ok(None);
    };
    let folder = mail.get_folder(account_id, &message.folder_id)?;
    if !matches!(folder.kind, MailFolderKind::Drafts) {
        return Ok(None);
    }

    Ok(Some(super::draft_ops::materialize_remote_draft(account_id, &message).await?))
}

pub(super) async fn save_draft(
    provider: &super::ImapSmtpProvider,
    draft: DraftMessage,
) -> Result<DraftMessage, BackendError> {
    if draft.account_id.trim().is_empty() {
        return Err(BackendError::validation("Account is required"));
    }

    let accounts = memory::store().accounts();
    let mail = memory::store().mail();
    let account_state = accounts.get_account_state(&draft.account_id)
        .ok_or_else(|| BackendError::not_found("Account not found"))?;
    let mailbox_name = super::folder_ops::get_drafts_mailbox_name(&draft.account_id)?;
    let raw_email = super::draft_ops::build_rfc822_message(&account_state, &draft)?.formatted();

    if let Some(existing_message) = mail.get_message(&draft.account_id, &draft.id)? {
        let existing_folder = mail.get_folder(&draft.account_id, &existing_message.folder_id)?;
        if matches!(existing_folder.kind, MailFolderKind::Drafts) {
            if let Some(uid) = existing_message.imap_uid {
                super::folder_ops::imap_delete_message_by_uid(&draft.account_id, &existing_folder.id, uid)
                    .await?;
            }
        }
    }

    let mut session = super::client_ops::imap_connect_by_account(&draft.account_id).await?;
    session
        .append(&mailbox_name, Some("(\\Draft)"), None, &raw_email)
        .await
        .map_err(|error| BackendError::internal(format!("IMAP APPEND draft failed: {error}")))?;
    let _ = session.logout().await;

    let _ = mail.remove_draft(&draft.account_id, &draft.id);
    provider.sync_account_cap(&draft.account_id).await?;

    if let Some(remote) = super::folder_ops::find_matching_remote_draft(&draft.account_id, &draft)? {
        return super::draft_ops::materialize_remote_draft(&draft.account_id, &remote).await;
    }

    Ok(draft)
}

pub(super) async fn send_message(draft: DraftMessage) -> Result<String, BackendError> {
    let account_state = prepare_smtp_send(&draft)?;
    let start = log_smtp_send_start(&draft, &account_state);
    let raw_email = super::smtp_send(&account_state, &draft).await?;
    finalize_smtp_send(draft, raw_email, start)
}

pub(super) async fn toggle_star(
    account_id: &str,
    message_id: &str,
) -> Result<Option<MailMessage>, BackendError> {
    let updated = memory::store().mail().toggle_star(account_id, message_id)?;
    if let Some(ref msg) = updated {
        if let Some(uid) = msg.imap_uid {
            eprintln!(
                "[imap] pushing star={} for uid {} in {}",
                msg.is_starred, uid, msg.folder_id
            );
            if let Err(e) =
                super::client_ops::imap_store_flag(account_id, &msg.folder_id, uid, "\\Flagged", msg.is_starred)
                    .await
            {
                eprintln!("[imap] push star failed: {}", e.message);
            }
        }
    }
    Ok(updated)
}

pub(super) async fn toggle_read(
    account_id: &str,
    message_id: &str,
) -> Result<Option<MailMessage>, BackendError> {
    let updated = memory::store().mail().toggle_read(account_id, message_id)?;
    if let Some(ref msg) = updated {
        if let Some(uid) = msg.imap_uid {
            eprintln!(
                "[imap] pushing read={} for uid {} in {}",
                msg.is_read, uid, msg.folder_id
            );
            if let Err(e) =
                super::client_ops::imap_store_flag(account_id, &msg.folder_id, uid, "\\Seen", msg.is_read).await
            {
                eprintln!("[imap] push read failed: {}", e.message);
            }
        }
    }
    Ok(updated)
}

pub(super) async fn delete_message(
    account_id: &str,
    message_id: &str,
) -> Result<(), BackendError> {
    let mail = memory::store().mail();
    let original = mail.get_message(account_id, message_id)?;
    mail.delete_message(account_id, message_id)?;

    if let Some(msg) = original {
        if let Some(uid) = msg.imap_uid {
            if let Ok(folders) = mail.list_folders(account_id) {
                if let Some(trash) = folders.iter().find(|f| matches!(f.kind, MailFolderKind::Trash))
                {
                    eprintln!("[imap] moving uid {} to trash", uid);
                    if let Err(e) =
                        super::folder_ops::imap_move_message(account_id, &msg.folder_id, &trash.id, uid).await
                    {
                        eprintln!("[imap] push delete failed: {}", e.message);
                    }
                }
            }
        }
    }
    Ok(())
}

pub(super) async fn archive_message(
    account_id: &str,
    message_id: &str,
) -> Result<Option<MailMessage>, BackendError> {
    let mail = memory::store().mail();
    let original = mail.get_message(account_id, message_id)?;
    let updated = mail.archive_message(account_id, message_id)?;

    if let (Some(orig), Some(ref upd)) = (original, &updated) {
        if let Some(uid) = orig.imap_uid {
            eprintln!(
                "[imap] archiving uid {} from {} to {}",
                uid, orig.folder_id, upd.folder_id
            );
            if let Err(e) =
                super::folder_ops::imap_move_message(account_id, &orig.folder_id, &upd.folder_id, uid).await
            {
                eprintln!("[imap] push archive failed: {}", e.message);
            }
        }
    }
    Ok(updated)
}

pub(super) async fn restore_message(
    account_id: &str,
    message_id: &str,
) -> Result<Option<MailMessage>, BackendError> {
    let mail = memory::store().mail();
    let original = mail.get_message(account_id, message_id)?;
    let updated = mail.restore_message(account_id, message_id)?;

    if let (Some(orig), Some(ref upd)) = (original, &updated) {
        if let Some(uid) = orig.imap_uid {
            eprintln!(
                "[imap] restoring uid {} from {} to {}",
                uid, orig.folder_id, upd.folder_id
            );
            if let Err(e) =
                super::folder_ops::imap_move_message(account_id, &orig.folder_id, &upd.folder_id, uid).await
            {
                eprintln!("[imap] push restore failed: {}", e.message);
            }
        }
    }
    Ok(updated)
}

pub(super) async fn move_message(
    account_id: &str,
    message_id: &str,
    folder_id: &str,
) -> Result<Option<MailMessage>, BackendError> {
    let mail = memory::store().mail();
    let original = mail.get_message(account_id, message_id)?;
    let updated = mail.move_message(account_id, message_id, folder_id)?;

    if let Some(orig) = original {
        if let Some(uid) = orig.imap_uid {
            eprintln!(
                "[imap] moving uid {} from {} to {}",
                uid, orig.folder_id, folder_id
            );
            if let Err(e) = super::folder_ops::imap_move_message(account_id, &orig.folder_id, folder_id, uid).await
            {
                eprintln!("[imap] push move failed: {}", e.message);
            }
        }
    }
    Ok(updated)
}

pub(super) async fn mark_all_read(account_id: &str, folder_id: &str) -> Result<(), BackendError> {
    let unread_uids: Vec<(u32, String)> = {
        let messages = memory::store().mail().list_messages(account_id, folder_id)?;
        messages
            .iter()
            .filter(|m| !m.is_read)
            .filter_map(|m| m.imap_uid.map(|uid| (uid, m.folder_id.clone())))
            .collect()
    };

    eprintln!(
        "[store] marking {} messages read in {folder_id}",
        unread_uids.len()
    );
    memory::store().mail().mark_all_read(account_id, folder_id)?;

    if !unread_uids.is_empty() {
        if let Some(real_folder_id) = unread_uids.first().map(|(_, fid)| fid.clone()) {
            if let Some(mailbox_name) = super::folder_ops::get_imap_folder_name(account_id, &real_folder_id) {
                eprintln!(
                    "[imap] pushing \\Seen for {} messages in {mailbox_name}",
                    unread_uids.len()
                );
                if let Ok(mut session) = super::client_ops::imap_connect_by_account(account_id).await {
                    if session.select(&mailbox_name).await.is_ok() {
                        for (uid, _) in &unread_uids {
                            if let Ok(stream) =
                                session.uid_store(uid.to_string(), "+FLAGS (\\Seen)").await
                            {
                                let _ = stream.try_collect::<Vec<_>>().await;
                            }
                        }
                    }
                    let _ = session.logout().await;
                }
            }
        }
    }

    Ok(())
}

pub(super) async fn get_attachment_content(
    account_id: &str,
    message_id: &str,
    attachment_id: &str,
) -> Result<AttachmentContent, BackendError> {
    get_attachment_content_from_storage(account_id, message_id, attachment_id)
}
