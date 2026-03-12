pub mod mock;
pub mod registry;

use crate::models::{
    AccountSetupDraft, DraftMessage, MailAccount, MailMessage, MailboxBundle, MailboxFolder,
    SyncStatus,
};

pub trait MailProvider: Sync {
    fn backend_name(&self) -> &'static str;
    fn list_accounts(&self) -> Vec<MailAccount>;
    fn create_account(&self, draft: AccountSetupDraft) -> MailAccount;
    fn list_folders(&self, account_id: &str) -> Vec<MailboxFolder>;
    fn list_messages(&self, account_id: &str, folder_id: &str) -> Vec<MailMessage>;
    fn get_message(&self, account_id: &str, message_id: &str) -> Option<MailMessage>;
    fn save_draft(&self, draft: DraftMessage) -> DraftMessage;
    fn send_message(&self, draft: DraftMessage) -> String;
    fn toggle_star(&self, account_id: &str, message_id: &str) -> Option<MailMessage>;
    fn toggle_read(&self, account_id: &str, message_id: &str) -> Option<MailMessage>;
    fn delete_message(&self, account_id: &str, message_id: &str);
    fn sync_account(&self, account_id: &str) -> SyncStatus;
    fn get_mailbox_bundle(&self, account_id: &str) -> MailboxBundle;
}
