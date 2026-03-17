use crate::provider::imap::IMAP_SMTP_PROVIDER;
use crate::provider::pop3::POP3_SMTP_PROVIDER;
use crate::provider::{
    AccountProvider, DraftProvider, FolderProvider, LabelProvider, MessageMutationProvider,
    MessageQueryProvider, SyncProvider,
};
use crate::storage::memory;

#[derive(Clone, Copy)]
pub struct ProviderRegistry;

impl ProviderRegistry {
    pub fn new() -> Self {
        Self
    }

    pub fn default_account_provider(&self) -> &'static dyn AccountProvider {
        &IMAP_SMTP_PROVIDER
    }

    pub fn account_provider_for_account(&self, account_id: &str) -> &'static dyn AccountProvider {
        if is_pop3_account(account_id) {
            &POP3_SMTP_PROVIDER
        } else {
            &IMAP_SMTP_PROVIDER
        }
    }

    pub fn account_provider_for_incoming_protocol(
        &self,
        protocol: &str,
    ) -> &'static dyn AccountProvider {
        if is_pop3_protocol(protocol) {
            &POP3_SMTP_PROVIDER
        } else {
            &IMAP_SMTP_PROVIDER
        }
    }

    pub fn folder_provider_for_account(&self, account_id: &str) -> &'static dyn FolderProvider {
        if is_pop3_account(account_id) {
            &POP3_SMTP_PROVIDER
        } else {
            &IMAP_SMTP_PROVIDER
        }
    }

    pub fn draft_provider_for_account(&self, account_id: &str) -> &'static dyn DraftProvider {
        if is_pop3_account(account_id) {
            &POP3_SMTP_PROVIDER
        } else {
            &IMAP_SMTP_PROVIDER
        }
    }

    pub fn message_query_provider_for_account(
        &self,
        account_id: &str,
    ) -> &'static dyn MessageQueryProvider {
        if is_pop3_account(account_id) {
            &POP3_SMTP_PROVIDER
        } else {
            &IMAP_SMTP_PROVIDER
        }
    }

    pub fn message_mutation_provider_for_account(
        &self,
        account_id: &str,
    ) -> &'static dyn MessageMutationProvider {
        if is_pop3_account(account_id) {
            &POP3_SMTP_PROVIDER
        } else {
            &IMAP_SMTP_PROVIDER
        }
    }

    pub fn label_provider_for_account(&self, account_id: &str) -> &'static dyn LabelProvider {
        if is_pop3_account(account_id) {
            &POP3_SMTP_PROVIDER
        } else {
            &IMAP_SMTP_PROVIDER
        }
    }

    pub fn sync_provider_for_account(&self, account_id: &str) -> &'static dyn SyncProvider {
        if is_pop3_account(account_id) {
            &POP3_SMTP_PROVIDER
        } else {
            &IMAP_SMTP_PROVIDER
        }
    }
}

pub fn default_provider_registry() -> ProviderRegistry {
    ProviderRegistry::new()
}

fn is_pop3_account(account_id: &str) -> bool {
    memory::store()
        .accounts()
        .get_account_state(account_id)
        .map(|state| state.config.incoming_protocol == "pop3")
        .unwrap_or(false)
}

fn is_pop3_protocol(protocol: &str) -> bool {
    protocol == "pop3"
}
