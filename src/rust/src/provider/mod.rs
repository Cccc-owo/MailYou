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
pub trait MailProvider: Send + Sync {
    fn backend_name(&self) -> &'static str;
    async fn list_accounts(&self) -> Result<Vec<MailAccount>, BackendError>;
    async fn create_account(&self, draft: AccountSetupDraft) -> Result<MailAccount, BackendError>;
    async fn test_account_connection(
        &self,
        draft: AccountSetupDraft,
    ) -> Result<SyncStatus, BackendError>;
    async fn list_folders(&self, account_id: &str) -> Result<Vec<MailboxFolder>, BackendError>;
    async fn create_folder(
        &self,
        account_id: &str,
        name: &str,
    ) -> Result<Vec<MailboxFolder>, BackendError>;
    async fn rename_folder(
        &self,
        account_id: &str,
        folder_id: &str,
        name: &str,
    ) -> Result<Vec<MailboxFolder>, BackendError>;
    async fn delete_folder(
        &self,
        account_id: &str,
        folder_id: &str,
    ) -> Result<Vec<MailboxFolder>, BackendError>;
    async fn list_messages(
        &self,
        account_id: &str,
        folder_id: &str,
    ) -> Result<Vec<MailMessage>, BackendError>;
    async fn get_draft(
        &self,
        account_id: &str,
        draft_id: &str,
    ) -> Result<Option<DraftMessage>, BackendError>;
    async fn search_messages(
        &self,
        account_id: &str,
        query: &str,
    ) -> Result<Vec<MailMessage>, BackendError>;
    async fn list_labels(&self, account_id: &str) -> Result<Vec<MailLabel>, BackendError>;
    async fn get_message(
        &self,
        account_id: &str,
        message_id: &str,
    ) -> Result<Option<MailMessage>, BackendError>;
    async fn add_label(
        &self,
        account_id: &str,
        message_id: &str,
        label: &str,
    ) -> Result<Option<MailMessage>, BackendError>;
    async fn remove_label(
        &self,
        account_id: &str,
        message_id: &str,
        label: &str,
    ) -> Result<Option<MailMessage>, BackendError>;
    async fn rename_label(
        &self,
        account_id: &str,
        label: &str,
        new_label: &str,
    ) -> Result<Vec<MailLabel>, BackendError>;
    async fn delete_label(
        &self,
        account_id: &str,
        label: &str,
    ) -> Result<Vec<MailLabel>, BackendError>;
    async fn save_draft(&self, draft: DraftMessage) -> Result<DraftMessage, BackendError>;
    async fn send_message(&self, draft: DraftMessage) -> Result<String, BackendError>;
    async fn toggle_star(
        &self,
        account_id: &str,
        message_id: &str,
    ) -> Result<Option<MailMessage>, BackendError>;
    async fn toggle_read(
        &self,
        account_id: &str,
        message_id: &str,
    ) -> Result<Option<MailMessage>, BackendError>;
    async fn delete_message(&self, account_id: &str, message_id: &str) -> Result<(), BackendError>;
    async fn delete_account(&self, account_id: &str) -> Result<(), BackendError>;
    async fn archive_message(
        &self,
        account_id: &str,
        message_id: &str,
    ) -> Result<Option<MailMessage>, BackendError>;
    async fn restore_message(
        &self,
        account_id: &str,
        message_id: &str,
    ) -> Result<Option<MailMessage>, BackendError>;
    async fn move_message(
        &self,
        account_id: &str,
        message_id: &str,
        folder_id: &str,
    ) -> Result<Option<MailMessage>, BackendError>;
    async fn mark_all_read(&self, account_id: &str, folder_id: &str) -> Result<(), BackendError>;
    async fn sync_account(&self, account_id: &str) -> Result<SyncStatus, BackendError>;
    async fn get_mailbox_bundle(&self, account_id: &str) -> Result<MailboxBundle, BackendError>;
    async fn get_attachment_content(
        &self,
        account_id: &str,
        message_id: &str,
        attachment_id: &str,
    ) -> Result<AttachmentContent, BackendError>;
    async fn get_account_config(&self, account_id: &str)
        -> Result<AccountSetupDraft, BackendError>;
    async fn update_account(
        &self,
        account_id: &str,
        draft: AccountSetupDraft,
    ) -> Result<MailAccount, BackendError>;
}
