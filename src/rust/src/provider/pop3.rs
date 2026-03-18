mod client;
mod mail_ops;

use async_trait::async_trait;

use crate::models::{
    AccountSetupDraft, AttachmentContent, DraftMessage, MailAccount, MailLabel, MailMessage,
    MailboxBundle, MailboxFolder, SyncStatus,
};
use crate::protocol::BackendError;
use crate::provider::common::validate_draft;
use crate::provider::{
    AccountProvider, DraftProvider, FolderProvider, LabelProvider, MessageMutationProvider,
    MessageQueryProvider, SyncProvider,
};
use crate::storage::memory;

pub struct Pop3SmtpProvider;

pub static POP3_SMTP_PROVIDER: Pop3SmtpProvider = Pop3SmtpProvider;

#[async_trait]
impl AccountProvider for Pop3SmtpProvider {
    async fn list_accounts_cap(&self) -> Result<Vec<MailAccount>, BackendError> {
        memory::store().accounts().list_accounts()
    }

    async fn create_account_cap(&self, draft: AccountSetupDraft) -> Result<MailAccount, BackendError> {
        eprintln!(
            "[pop3] testing connection for new account {}...",
            draft.email
        );
        self.test_account_connection_cap(draft.clone()).await?;
        eprintln!("[pop3] connection test passed, creating account");
        memory::store().accounts().create_account_without_test(draft)
    }

    async fn test_account_connection_cap(
        &self,
        draft: AccountSetupDraft,
    ) -> Result<SyncStatus, BackendError> {
        validate_draft(&draft)?;
        eprintln!(
            "[pop3] connecting to {}:{}...",
            draft.incoming_host, draft.incoming_port
        );
        mail_ops::test_account_connection(draft).await
    }

    async fn delete_account_cap(&self, account_id: &str) -> Result<(), BackendError> {
        eprintln!("[store] deleting account {account_id}");
        memory::store().accounts().delete_account(account_id)
    }

    async fn get_account_config_cap(
        &self,
        account_id: &str,
    ) -> Result<AccountSetupDraft, BackendError> {
        memory::store().accounts().get_account_config(account_id)
    }

    async fn update_account_cap(
        &self,
        account_id: &str,
        draft: AccountSetupDraft,
    ) -> Result<MailAccount, BackendError> {
        memory::store().accounts().update_account(account_id, draft)
    }
}

#[async_trait]
impl FolderProvider for Pop3SmtpProvider {
    async fn list_folders_cap(&self, account_id: &str) -> Result<Vec<MailboxFolder>, BackendError> {
        memory::store().mail().list_folders(account_id)
    }

    async fn create_folder_cap(
        &self,
        _account_id: &str,
        _name: &str,
    ) -> Result<Vec<MailboxFolder>, BackendError> {
        Err(BackendError::validation(
            "POP3 accounts do not support server folders",
        ))
    }

    async fn rename_folder_cap(
        &self,
        _account_id: &str,
        _folder_id: &str,
        _name: &str,
    ) -> Result<Vec<MailboxFolder>, BackendError> {
        Err(BackendError::validation(
            "POP3 accounts do not support server folders",
        ))
    }

    async fn delete_folder_cap(
        &self,
        _account_id: &str,
        _folder_id: &str,
    ) -> Result<Vec<MailboxFolder>, BackendError> {
        Err(BackendError::validation(
            "POP3 accounts do not support server folders",
        ))
    }
}

#[async_trait]
impl MessageQueryProvider for Pop3SmtpProvider {
    async fn list_messages_cap(
        &self,
        account_id: &str,
        folder_id: &str,
    ) -> Result<Vec<MailMessage>, BackendError> {
        memory::store().mail().list_messages(account_id, folder_id)
    }

    async fn search_messages_cap(
        &self,
        account_id: &str,
        query: &str,
    ) -> Result<Vec<MailMessage>, BackendError> {
        memory::store().mail().search_messages(account_id, query)
    }

    async fn get_message_cap(
        &self,
        account_id: &str,
        message_id: &str,
    ) -> Result<Option<MailMessage>, BackendError> {
        memory::store().mail().get_message(account_id, message_id)
    }

    async fn get_attachment_content_cap(
        &self,
        account_id: &str,
        message_id: &str,
        attachment_id: &str,
    ) -> Result<AttachmentContent, BackendError> {
        mail_ops::get_attachment_content(account_id, message_id, attachment_id).await
    }
}

#[async_trait]
impl DraftProvider for Pop3SmtpProvider {
    async fn get_draft_cap(
        &self,
        account_id: &str,
        draft_id: &str,
    ) -> Result<Option<DraftMessage>, BackendError> {
        memory::store().mail().get_draft(account_id, draft_id)
    }

    async fn save_draft_cap(&self, draft: DraftMessage) -> Result<DraftMessage, BackendError> {
        memory::store().mail().save_draft(draft)
    }

    async fn send_message_cap(&self, draft: DraftMessage) -> Result<String, BackendError> {
        mail_ops::send_message(draft).await
    }
}

#[async_trait]
impl MessageMutationProvider for Pop3SmtpProvider {
    async fn toggle_star_cap(
        &self,
        account_id: &str,
        message_id: &str,
    ) -> Result<Option<MailMessage>, BackendError> {
        memory::store().mail().toggle_star(account_id, message_id)
    }

    async fn toggle_read_cap(
        &self,
        account_id: &str,
        message_id: &str,
    ) -> Result<Option<MailMessage>, BackendError> {
        memory::store().mail().toggle_read(account_id, message_id)
    }

    async fn batch_toggle_read_cap(
        &self,
        account_id: &str,
        message_ids: &[String],
        is_read: bool,
    ) -> Result<(), BackendError> {
        for message_id in message_ids {
            let current = memory::store().mail().get_message(account_id, message_id)?;
            if current.as_ref().map(|message| message.is_read) != Some(is_read) {
                let _ = memory::store().mail().toggle_read(account_id, message_id)?;
            }
        }
        Ok(())
    }

    async fn delete_message_cap(&self, account_id: &str, message_id: &str) -> Result<(), BackendError> {
        memory::store().mail().delete_message(account_id, message_id)
    }

    async fn batch_delete_messages_cap(
        &self,
        account_id: &str,
        message_ids: &[String],
    ) -> Result<(), BackendError> {
        for message_id in message_ids {
            memory::store().mail().delete_message(account_id, message_id)?;
        }
        Ok(())
    }

    async fn archive_message_cap(
        &self,
        account_id: &str,
        message_id: &str,
    ) -> Result<Option<MailMessage>, BackendError> {
        memory::store().mail().archive_message(account_id, message_id)
    }

    async fn restore_message_cap(
        &self,
        account_id: &str,
        message_id: &str,
    ) -> Result<Option<MailMessage>, BackendError> {
        memory::store().mail().restore_message(account_id, message_id)
    }

    async fn move_message_cap(
        &self,
        account_id: &str,
        message_id: &str,
        folder_id: &str,
    ) -> Result<Option<MailMessage>, BackendError> {
        memory::store().mail().move_message(account_id, message_id, folder_id)
    }

    async fn batch_move_messages_cap(
        &self,
        account_id: &str,
        message_ids: &[String],
        folder_id: &str,
    ) -> Result<(), BackendError> {
        for message_id in message_ids {
            let _ = memory::store().mail().move_message(account_id, message_id, folder_id)?;
        }
        Ok(())
    }

    async fn mark_all_read_cap(&self, account_id: &str, folder_id: &str) -> Result<(), BackendError> {
        memory::store().mail().mark_all_read(account_id, folder_id)
    }
}

#[async_trait]
impl LabelProvider for Pop3SmtpProvider {
    async fn list_labels_cap(&self, account_id: &str) -> Result<Vec<MailLabel>, BackendError> {
        let _ = account_id;
        Ok(Vec::new())
    }

    async fn add_label_cap(
        &self,
        account_id: &str,
        message_id: &str,
        label: &str,
    ) -> Result<Option<MailMessage>, BackendError> {
        let _ = (account_id, message_id, label);
        Err(BackendError::validation(
            "POP3 accounts do not support server labels",
        ))
    }

    async fn remove_label_cap(
        &self,
        account_id: &str,
        message_id: &str,
        label: &str,
    ) -> Result<Option<MailMessage>, BackendError> {
        let _ = (account_id, message_id, label);
        Err(BackendError::validation(
            "POP3 accounts do not support server labels",
        ))
    }

    async fn rename_label_cap(
        &self,
        account_id: &str,
        label: &str,
        new_label: &str,
    ) -> Result<Vec<MailLabel>, BackendError> {
        let _ = (account_id, label, new_label);
        Err(BackendError::validation(
            "POP3 accounts do not support server labels",
        ))
    }

    async fn delete_label_cap(
        &self,
        account_id: &str,
        label: &str,
    ) -> Result<Vec<MailLabel>, BackendError> {
        let _ = (account_id, label);
        Err(BackendError::validation(
            "POP3 accounts do not support server labels",
        ))
    }
}

#[async_trait]
impl SyncProvider for Pop3SmtpProvider {
    async fn sync_account_cap(&self, account_id: &str) -> Result<SyncStatus, BackendError> {
        mail_ops::sync_account(account_id).await
    }

    async fn get_mailbox_bundle_cap(&self, account_id: &str) -> Result<MailboxBundle, BackendError> {
        memory::store().mail().get_mailbox_bundle(account_id)
    }
}
