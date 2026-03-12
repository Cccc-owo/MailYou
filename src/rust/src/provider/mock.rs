use crate::models::{
    AccountSetupDraft, DraftMessage, MailAccount, MailMessage, MailboxBundle, MailboxFolder,
    SyncStatus,
};
use crate::provider::MailProvider;
use crate::storage::memory;

pub struct MockMailProvider;

impl MailProvider for MockMailProvider {
    fn backend_name(&self) -> &'static str {
        memory::backend_name()
    }

    fn list_accounts(&self) -> Vec<MailAccount> {
        memory::accounts()
    }

    fn create_account(&self, draft: AccountSetupDraft) -> MailAccount {
        memory::create_account(draft)
    }

    fn list_folders(&self, account_id: &str) -> Vec<MailboxFolder> {
        memory::folders(account_id)
    }

    fn list_messages(&self, account_id: &str, folder_id: &str) -> Vec<MailMessage> {
        memory::messages(account_id, folder_id)
    }

    fn get_message(&self, account_id: &str, message_id: &str) -> Option<MailMessage> {
        memory::get_message(account_id, message_id)
    }

    fn save_draft(&self, draft: DraftMessage) -> DraftMessage {
        memory::save_draft(draft)
    }

    fn send_message(&self, draft: DraftMessage) -> String {
        memory::send_message(draft)
    }

    fn toggle_star(&self, account_id: &str, message_id: &str) -> Option<MailMessage> {
        memory::toggle_star(account_id, message_id)
    }

    fn toggle_read(&self, account_id: &str, message_id: &str) -> Option<MailMessage> {
        memory::toggle_read(account_id, message_id)
    }

    fn delete_message(&self, account_id: &str, message_id: &str) {
        memory::delete_message(account_id, message_id)
    }

    fn sync_account(&self, account_id: &str) -> SyncStatus {
        memory::sync_status(account_id)
    }

    fn get_mailbox_bundle(&self, account_id: &str) -> MailboxBundle {
        memory::mailbox_bundle(account_id)
    }
}

pub static MOCK_PROVIDER: MockMailProvider = MockMailProvider;
