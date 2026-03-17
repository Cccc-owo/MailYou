pub mod common;
pub mod imap;
pub mod pop3;
pub mod registry;

use async_trait::async_trait;

use crate::models::{
    AccountSetupDraft, AttachmentContent, DraftMessage, MailAccount, MailLabel, MailMessage,
    MailboxBundle, MailboxFolder, SyncStatus,
};
use crate::protocol::BackendError;

#[async_trait]
pub trait AccountProvider: Send + Sync {
    async fn list_accounts_cap(&self) -> Result<Vec<MailAccount>, BackendError>;
    async fn create_account_cap(
        &self,
        draft: AccountSetupDraft,
    ) -> Result<MailAccount, BackendError>;
    async fn test_account_connection_cap(
        &self,
        draft: AccountSetupDraft,
    ) -> Result<SyncStatus, BackendError>;
    async fn delete_account_cap(&self, account_id: &str) -> Result<(), BackendError>;
    async fn get_account_config_cap(
        &self,
        account_id: &str,
    ) -> Result<AccountSetupDraft, BackendError>;
    async fn update_account_cap(
        &self,
        account_id: &str,
        draft: AccountSetupDraft,
    ) -> Result<MailAccount, BackendError>;
}

#[async_trait]
pub trait FolderProvider: Send + Sync {
    async fn list_folders_cap(&self, account_id: &str) -> Result<Vec<MailboxFolder>, BackendError>;
    async fn create_folder_cap(
        &self,
        account_id: &str,
        name: &str,
    ) -> Result<Vec<MailboxFolder>, BackendError>;
    async fn rename_folder_cap(
        &self,
        account_id: &str,
        folder_id: &str,
        name: &str,
    ) -> Result<Vec<MailboxFolder>, BackendError>;
    async fn delete_folder_cap(
        &self,
        account_id: &str,
        folder_id: &str,
    ) -> Result<Vec<MailboxFolder>, BackendError>;
}

#[async_trait]
pub trait DraftProvider: Send + Sync {
    async fn get_draft_cap(
        &self,
        account_id: &str,
        draft_id: &str,
    ) -> Result<Option<DraftMessage>, BackendError>;
    async fn save_draft_cap(&self, draft: DraftMessage) -> Result<DraftMessage, BackendError>;
    async fn send_message_cap(&self, draft: DraftMessage) -> Result<String, BackendError>;
}

#[async_trait]
pub trait MessageQueryProvider: Send + Sync {
    async fn list_messages_cap(
        &self,
        account_id: &str,
        folder_id: &str,
    ) -> Result<Vec<MailMessage>, BackendError>;
    async fn search_messages_cap(
        &self,
        account_id: &str,
        query: &str,
    ) -> Result<Vec<MailMessage>, BackendError>;
    async fn get_message_cap(
        &self,
        account_id: &str,
        message_id: &str,
    ) -> Result<Option<MailMessage>, BackendError>;
    async fn get_attachment_content_cap(
        &self,
        account_id: &str,
        message_id: &str,
        attachment_id: &str,
    ) -> Result<AttachmentContent, BackendError>;
}

#[async_trait]
pub trait MessageMutationProvider: Send + Sync {
    async fn toggle_star_cap(
        &self,
        account_id: &str,
        message_id: &str,
    ) -> Result<Option<MailMessage>, BackendError>;
    async fn toggle_read_cap(
        &self,
        account_id: &str,
        message_id: &str,
    ) -> Result<Option<MailMessage>, BackendError>;
    async fn delete_message_cap(
        &self,
        account_id: &str,
        message_id: &str,
    ) -> Result<(), BackendError>;
    async fn archive_message_cap(
        &self,
        account_id: &str,
        message_id: &str,
    ) -> Result<Option<MailMessage>, BackendError>;
    async fn restore_message_cap(
        &self,
        account_id: &str,
        message_id: &str,
    ) -> Result<Option<MailMessage>, BackendError>;
    async fn move_message_cap(
        &self,
        account_id: &str,
        message_id: &str,
        folder_id: &str,
    ) -> Result<Option<MailMessage>, BackendError>;
    async fn mark_all_read_cap(&self, account_id: &str, folder_id: &str) -> Result<(), BackendError>;
}

#[async_trait]
pub trait LabelProvider: Send + Sync {
    async fn list_labels_cap(&self, account_id: &str) -> Result<Vec<MailLabel>, BackendError>;
    async fn add_label_cap(
        &self,
        account_id: &str,
        message_id: &str,
        label: &str,
    ) -> Result<Option<MailMessage>, BackendError>;
    async fn remove_label_cap(
        &self,
        account_id: &str,
        message_id: &str,
        label: &str,
    ) -> Result<Option<MailMessage>, BackendError>;
    async fn rename_label_cap(
        &self,
        account_id: &str,
        label: &str,
        new_label: &str,
    ) -> Result<Vec<MailLabel>, BackendError>;
    async fn delete_label_cap(
        &self,
        account_id: &str,
        label: &str,
    ) -> Result<Vec<MailLabel>, BackendError>;
}

#[async_trait]
pub trait SyncProvider: Send + Sync {
    async fn sync_account_cap(&self, account_id: &str) -> Result<SyncStatus, BackendError>;
    async fn get_mailbox_bundle_cap(
        &self,
        account_id: &str,
    ) -> Result<MailboxBundle, BackendError>;
}
