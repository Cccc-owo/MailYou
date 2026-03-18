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
    let original = memory::store().mail().get_message(account_id, message_id)?;
    if let Some(ref msg) = original {
        if let Some(uid) = msg.imap_uid {
            eprintln!(
                "[imap] pushing star={} for uid {} in {}",
                !msg.is_starred, uid, msg.folder_id
            );
            if let Err(error) =
                super::client_ops::imap_store_flag(account_id, &msg.folder_id, uid, "\\Flagged", !msg.is_starred)
                    .await
            {
                eprintln!("[imap] push star failed: {}", error.message);
                return Err(error);
            }
        }
    }
    memory::store().mail().toggle_star(account_id, message_id)
}

pub(super) async fn toggle_read(
    account_id: &str,
    message_id: &str,
) -> Result<Option<MailMessage>, BackendError> {
    let original = memory::store().mail().get_message(account_id, message_id)?;
    if let Some(ref msg) = original {
        if let Some(uid) = msg.imap_uid {
            eprintln!(
                "[imap] pushing read={} for uid {} in {}",
                !msg.is_read, uid, msg.folder_id
            );
            if let Err(error) =
                super::client_ops::imap_store_flag(account_id, &msg.folder_id, uid, "\\Seen", !msg.is_read).await
            {
                eprintln!("[imap] push read failed: {}", error.message);
                return Err(error);
            }
        }
    }
    memory::store().mail().toggle_read(account_id, message_id)
}

pub(super) async fn batch_toggle_read(
    account_id: &str,
    message_ids: &[String],
    is_read: bool,
) -> Result<(), BackendError> {
    if message_ids.is_empty() {
        return Ok(());
    }

    let mail = memory::store().mail();
    let originals = message_ids
        .iter()
        .filter_map(|message_id| mail.get_message(account_id, message_id).ok().flatten())
        .collect::<Vec<_>>();

    let mut changed_ids = Vec::<String>::new();
    let mut uids_by_folder = std::collections::HashMap::<String, Vec<u32>>::new();
    for message in originals {
        if message.is_read == is_read {
            continue;
        }
        changed_ids.push(message.id.clone());
        if let Some(uid) = message.imap_uid {
            uids_by_folder.entry(message.folder_id).or_default().push(uid);
        }
    }

    let mut pushed_folders = Vec::<(String, Vec<u32>)>::new();
    for (folder_id, uids) in uids_by_folder {
        eprintln!(
            "[imap] pushing read={} for {} messages in {}",
            is_read,
            uids.len(),
            folder_id
        );
        if let Err(error) =
            super::client_ops::imap_store_flags(account_id, &folder_id, &uids, "\\Seen", is_read).await
        {
            for (pushed_folder_id, pushed_uids) in pushed_folders.iter().rev() {
                if let Err(revert_error) = super::client_ops::imap_store_flags(
                    account_id,
                    pushed_folder_id,
                    pushed_uids,
                    "\\Seen",
                    !is_read,
                )
                .await
                {
                    eprintln!(
                        "[imap] batch push read revert failed for {}: {}",
                        pushed_folder_id, revert_error.message
                    );
                }
            }
            eprintln!("[imap] batch push read failed: {}", error.message);
            return Err(error);
        }
        pushed_folders.push((folder_id, uids));
    }

    if !changed_ids.is_empty() {
        memory::store().mail().batch_toggle_read(account_id, &changed_ids, is_read)?;
    }

    Ok(())
}

pub(super) async fn delete_message(
    account_id: &str,
    message_id: &str,
) -> Result<(), BackendError> {
    let mail = memory::store().mail();
    let pending_ids = vec![message_id.to_string()];
    let original = mail.get_message(account_id, message_id)?;
    let original_folder = original
        .as_ref()
        .map(|message| mail.get_folder(account_id, &message.folder_id))
        .transpose()?;
    mail.mark_pending_deleted_messages(account_id, &pending_ids);

    if let Some(msg) = original {
        if let Some(uid) = msg.imap_uid {
            if matches!(original_folder.as_ref().map(|folder| &folder.kind), Some(MailFolderKind::Trash)) {
                eprintln!("[imap] permanently deleting uid {} from trash", uid);
                if let Err(error) =
                    super::folder_ops::imap_delete_message_by_uid(account_id, &msg.folder_id, uid).await
                {
                    mail.clear_pending_deleted_messages(account_id, &pending_ids);
                    eprintln!("[imap] push permanent delete failed: {}", error.message);
                    return Err(error);
                }
            } else if let Ok(folders) = mail.list_folders(account_id) {
                if let Some(trash) = folders.iter().find(|f| matches!(f.kind, MailFolderKind::Trash)) {
                    eprintln!("[imap] moving uid {} to trash", uid);
                    if let Err(error) =
                        super::folder_ops::imap_move_message(account_id, &msg.folder_id, &trash.id, uid).await
                    {
                        mail.clear_pending_deleted_messages(account_id, &pending_ids);
                        eprintln!("[imap] push delete failed: {}", error.message);
                        return Err(error);
                    }
                }
            }
        }
    }
    mail.delete_message(account_id, message_id)?;
    mail.clear_pending_deleted_messages(account_id, &pending_ids);
    Ok(())
}

pub(super) async fn batch_delete_messages(
    account_id: &str,
    message_ids: &[String],
) -> Result<(), BackendError> {
    if message_ids.is_empty() {
        return Ok(());
    }

    let mail = memory::store().mail();
    let originals = message_ids
        .iter()
        .filter_map(|message_id| mail.get_message(account_id, message_id).ok().flatten())
        .collect::<Vec<_>>();
    mail.mark_pending_deleted_messages(account_id, message_ids);

    let folders = mail.list_folders(account_id).unwrap_or_default();
    let trash_folder = folders
        .iter()
        .find(|folder| matches!(folder.kind, MailFolderKind::Trash))
        .cloned();

    let mut delete_from_trash = std::collections::HashMap::<String, Vec<u32>>::new();
    let mut move_to_trash = std::collections::HashMap::<String, Vec<u32>>::new();

    for message in originals {
        let Some(uid) = message.imap_uid else {
            continue;
        };
        let folder_kind = folders
            .iter()
            .find(|folder| folder.id == message.folder_id)
            .map(|folder| folder.kind.clone());

        if matches!(folder_kind, Some(MailFolderKind::Trash)) {
            delete_from_trash.entry(message.folder_id).or_default().push(uid);
        } else {
            move_to_trash.entry(message.folder_id).or_default().push(uid);
        }
    }

    let mut moved_to_trash = Vec::<(String, Vec<u32>)>::new();
    if let Some(trash_folder) = trash_folder {
        for (folder_id, uids) in move_to_trash {
            eprintln!(
                "[imap] moving {} messages from {} to trash",
                uids.len(),
                folder_id
            );
            if let Err(error) =
                super::folder_ops::imap_move_messages(account_id, &folder_id, &trash_folder.id, &uids).await
            {
                for (moved_folder_id, moved_uids) in moved_to_trash.iter().rev() {
                    if let Err(revert_error) = super::folder_ops::imap_move_messages(
                        account_id,
                        &trash_folder.id,
                        moved_folder_id,
                        moved_uids,
                    )
                    .await
                    {
                        eprintln!(
                            "[imap] batch push delete revert failed for {}: {}",
                            moved_folder_id, revert_error.message
                        );
                    }
                }
                mail.clear_pending_deleted_messages(account_id, message_ids);
                eprintln!("[imap] batch push delete failed: {}", error.message);
                return Err(error);
            }
            moved_to_trash.push((folder_id, uids));
        }
    }

    for (folder_id, uids) in delete_from_trash {
        eprintln!("[imap] permanently deleting {} messages from {}", uids.len(), folder_id);
        if let Err(error) =
            super::folder_ops::imap_delete_messages_by_uid(account_id, &folder_id, &uids).await
        {
            if let Some(trash_folder) = folders
                .iter()
                .find(|folder| matches!(folder.kind, MailFolderKind::Trash))
            {
                for (moved_folder_id, moved_uids) in moved_to_trash.iter().rev() {
                    if let Err(revert_error) = super::folder_ops::imap_move_messages(
                        account_id,
                        &trash_folder.id,
                        moved_folder_id,
                        moved_uids,
                    )
                    .await
                    {
                        eprintln!(
                            "[imap] batch push delete revert failed for {}: {}",
                            moved_folder_id, revert_error.message
                        );
                    }
                }
            }
            mail.clear_pending_deleted_messages(account_id, message_ids);
            eprintln!("[imap] batch permanent delete failed: {}", error.message);
            return Err(error);
        }
    }

    memory::store().mail().batch_delete_messages(account_id, message_ids)?;
    mail.clear_pending_deleted_messages(account_id, message_ids);
    Ok(())
}

pub(super) async fn archive_message(
    account_id: &str,
    message_id: &str,
) -> Result<Option<MailMessage>, BackendError> {
    let mail = memory::store().mail();
    let original = mail.get_message(account_id, message_id)?;
    if let Some(orig) = original.as_ref() {
        if let Some(uid) = orig.imap_uid {
            let archive_folder_id = mail
                .list_folders(account_id)?
                .into_iter()
                .find(|folder| matches!(folder.kind, MailFolderKind::Archive))
                .map(|folder| folder.id)
                .ok_or_else(|| BackendError::internal("Archive folder is missing"))?;
            eprintln!(
                "[imap] archiving uid {} from {} to {}",
                uid, orig.folder_id, archive_folder_id
            );
            if let Err(error) =
                super::folder_ops::imap_move_message(account_id, &orig.folder_id, &archive_folder_id, uid).await
            {
                eprintln!("[imap] push archive failed: {}", error.message);
                return Err(error);
            }
        }
    }
    mail.archive_message(account_id, message_id)
}

pub(super) async fn restore_message(
    account_id: &str,
    message_id: &str,
) -> Result<Option<MailMessage>, BackendError> {
    let mail = memory::store().mail();
    let original = mail.get_message(account_id, message_id)?;
    if let Some(orig) = original.as_ref() {
        if let Some(uid) = orig.imap_uid {
            let inbox_folder_id = mail
                .list_folders(account_id)?
                .into_iter()
                .find(|folder| matches!(folder.kind, MailFolderKind::Inbox))
                .map(|folder| folder.id)
                .ok_or_else(|| BackendError::internal("Inbox folder is missing"))?;
            let restore_folder_id = match orig.previous_folder_id.clone() {
                Some(previous_folder_id) if mail.get_folder(account_id, &previous_folder_id).is_ok() => previous_folder_id,
                _ => inbox_folder_id,
            };
            eprintln!(
                "[imap] restoring uid {} from {} to {}",
                uid, orig.folder_id, restore_folder_id
            );
            if let Err(error) =
                super::folder_ops::imap_move_message(account_id, &orig.folder_id, &restore_folder_id, uid).await
            {
                eprintln!("[imap] push restore failed: {}", error.message);
                return Err(error);
            }
            return mail.restore_message(account_id, message_id);
        }
    }
    mail.restore_message(account_id, message_id)
}

pub(super) async fn move_message(
    account_id: &str,
    message_id: &str,
    folder_id: &str,
) -> Result<Option<MailMessage>, BackendError> {
    let mail = memory::store().mail();
    let original = mail.get_message(account_id, message_id)?;
    if let Some(orig) = original.as_ref() {
        if let Some(uid) = orig.imap_uid {
            eprintln!(
                "[imap] moving uid {} from {} to {}",
                uid, orig.folder_id, folder_id
            );
            if let Err(error) = super::folder_ops::imap_move_message(account_id, &orig.folder_id, folder_id, uid).await
            {
                eprintln!("[imap] push move failed: {}", error.message);
                return Err(error);
            }
        }
    }
    mail.move_message(account_id, message_id, folder_id)
}

pub(super) async fn batch_move_messages(
    account_id: &str,
    message_ids: &[String],
    folder_id: &str,
) -> Result<(), BackendError> {
    if message_ids.is_empty() {
        return Ok(());
    }

    let mail = memory::store().mail();
    let originals = message_ids
        .iter()
        .filter_map(|message_id| mail.get_message(account_id, message_id).ok().flatten())
        .collect::<Vec<_>>();

    let mut uids_by_folder = std::collections::HashMap::<String, Vec<u32>>::new();
    for message in originals {
        if let Some(uid) = message.imap_uid {
            uids_by_folder.entry(message.folder_id).or_default().push(uid);
        }
    }

    let mut moved_folders = Vec::<(String, Vec<u32>)>::new();
    for (source_folder_id, uids) in uids_by_folder {
        eprintln!(
            "[imap] moving {} messages from {} to {}",
            uids.len(),
            source_folder_id,
            folder_id
        );
        if let Err(error) =
            super::folder_ops::imap_move_messages(account_id, &source_folder_id, folder_id, &uids).await
        {
            for (moved_source_folder_id, moved_uids) in moved_folders.iter().rev() {
                if let Err(revert_error) =
                    super::folder_ops::imap_move_messages(account_id, folder_id, moved_source_folder_id, moved_uids)
                        .await
                {
                    eprintln!(
                        "[imap] batch push move revert failed for {}: {}",
                        moved_source_folder_id, revert_error.message
                    );
                }
            }
            eprintln!("[imap] batch push move failed: {}", error.message);
            return Err(error);
        }
        moved_folders.push((source_folder_id, uids));
    }

    memory::store().mail().batch_move_messages(account_id, message_ids, folder_id)?;
    Ok(())
}

pub(super) async fn mark_all_read(account_id: &str, folder_id: &str) -> Result<(), BackendError> {
    let unread_message_ids: Vec<String> = {
        let messages = memory::store().mail().list_messages(account_id, folder_id)?;
        messages
            .iter()
            .filter(|m| !m.is_read)
            .map(|m| m.id.clone())
            .collect()
    };

    eprintln!(
        "[store] marking {} messages read in {folder_id}",
        unread_message_ids.len()
    );
    batch_toggle_read(account_id, &unread_message_ids, true).await
}

pub(super) async fn get_attachment_content(
    account_id: &str,
    message_id: &str,
    attachment_id: &str,
) -> Result<AttachmentContent, BackendError> {
    get_attachment_content_from_storage(account_id, message_id, attachment_id)
}
