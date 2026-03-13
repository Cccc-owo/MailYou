pub mod imap;
pub mod registry;

use crate::models::{
    AccountSetupDraft, DraftMessage, MailAccount, MailMessage, MailboxBundle, MailboxFolder,
    SyncStatus,
};
use crate::protocol::BackendError;

pub trait MailProvider: Sync {
    fn backend_name(&self) -> &'static str;
    fn list_accounts(&self) -> Result<Vec<MailAccount>, BackendError>;
    fn create_account(&self, draft: AccountSetupDraft) -> Result<MailAccount, BackendError>;
    fn test_account_connection(&self, draft: AccountSetupDraft) -> Result<SyncStatus, BackendError>;
    fn list_folders(&self, account_id: &str) -> Result<Vec<MailboxFolder>, BackendError>;
    fn list_messages(&self, account_id: &str, folder_id: &str) -> Result<Vec<MailMessage>, BackendError>;
    fn get_message(&self, account_id: &str, message_id: &str) -> Result<Option<MailMessage>, BackendError>;
    fn save_draft(&self, draft: DraftMessage) -> Result<DraftMessage, BackendError>;
    fn send_message(&self, draft: DraftMessage) -> Result<String, BackendError>;
    fn toggle_star(&self, account_id: &str, message_id: &str) -> Result<Option<MailMessage>, BackendError>;
    fn toggle_read(&self, account_id: &str, message_id: &str) -> Result<Option<MailMessage>, BackendError>;
    fn delete_message(&self, account_id: &str, message_id: &str) -> Result<(), BackendError>;
    fn delete_account(&self, account_id: &str) -> Result<(), BackendError>;
    fn archive_message(&self, account_id: &str, message_id: &str) -> Result<Option<MailMessage>, BackendError>;
    fn restore_message(&self, account_id: &str, message_id: &str) -> Result<Option<MailMessage>, BackendError>;
    fn move_message(&self, account_id: &str, message_id: &str, folder_id: &str) -> Result<Option<MailMessage>, BackendError>;
    fn mark_all_read(&self, account_id: &str, folder_id: &str) -> Result<(), BackendError>;
    fn sync_account(&self, account_id: &str) -> Result<SyncStatus, BackendError>;
    fn get_mailbox_bundle(&self, account_id: &str) -> Result<MailboxBundle, BackendError>;
}
