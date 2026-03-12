use crate::models::{
    AccountSetupDraft, DraftMessage, MailAccount, MailMessage, MailboxBundle, MailboxFolder,
    SyncStatus,
};
use crate::protocol::BackendError;
use crate::provider::MailProvider;
use crate::storage::memory;

pub struct MockMailProvider;

impl MailProvider for MockMailProvider {
    fn backend_name(&self) -> &'static str {
        memory::backend_name()
    }

    fn list_accounts(&self) -> Result<Vec<MailAccount>, BackendError> {
        memory::list_accounts()
    }

    fn create_account(&self, draft: AccountSetupDraft) -> Result<MailAccount, BackendError> {
        memory::create_account(draft)
    }

    fn test_account_connection(&self, draft: AccountSetupDraft) -> Result<SyncStatus, BackendError> {
        memory::test_account_connection(draft)
    }

    fn list_folders(&self, account_id: &str) -> Result<Vec<MailboxFolder>, BackendError> {
        memory::list_folders(account_id)
    }

    fn list_messages(&self, account_id: &str, folder_id: &str) -> Result<Vec<MailMessage>, BackendError> {
        memory::list_messages(account_id, folder_id)
    }

    fn get_message(&self, account_id: &str, message_id: &str) -> Result<Option<MailMessage>, BackendError> {
        memory::get_message(account_id, message_id)
    }

    fn save_draft(&self, draft: DraftMessage) -> Result<DraftMessage, BackendError> {
        memory::save_draft(draft)
    }

    fn send_message(&self, draft: DraftMessage) -> Result<String, BackendError> {
        memory::send_message(draft)
    }

    fn toggle_star(&self, account_id: &str, message_id: &str) -> Result<Option<MailMessage>, BackendError> {
        memory::toggle_star(account_id, message_id)
    }

    fn toggle_read(&self, account_id: &str, message_id: &str) -> Result<Option<MailMessage>, BackendError> {
        memory::toggle_read(account_id, message_id)
    }

    fn delete_message(&self, account_id: &str, message_id: &str) -> Result<(), BackendError> {
        memory::delete_message(account_id, message_id)
    }

    fn archive_message(&self, account_id: &str, message_id: &str) -> Result<Option<MailMessage>, BackendError> {
        memory::archive_message(account_id, message_id)
    }

    fn restore_message(&self, account_id: &str, message_id: &str) -> Result<Option<MailMessage>, BackendError> {
        memory::restore_message(account_id, message_id)
    }

    fn move_message(&self, account_id: &str, message_id: &str, folder_id: &str) -> Result<Option<MailMessage>, BackendError> {
        memory::move_message(account_id, message_id, folder_id)
    }

    fn sync_account(&self, account_id: &str) -> Result<SyncStatus, BackendError> {
        memory::sync_account(account_id)
    }

    fn get_mailbox_bundle(&self, account_id: &str) -> Result<MailboxBundle, BackendError> {
        memory::get_mailbox_bundle(account_id)
    }
}

pub static MOCK_PROVIDER: MockMailProvider = MockMailProvider;
