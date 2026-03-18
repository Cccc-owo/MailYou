use crate::oauth::list_oauth_providers;
use crate::protocol::{serialize, BackendError, BackendRequest, HealthCheckResult, SendMessageResult};
use crate::provider::{
    AccountProvider, DraftProvider, FolderProvider, LabelProvider, MessageMutationProvider,
    MessageQueryProvider, SyncProvider,
};
use crate::provider::registry::ProviderRegistry;

pub struct MailService<'a> {
    registry: &'a ProviderRegistry,
}

impl<'a> MailService<'a> {
    pub fn new(registry: &'a ProviderRegistry) -> Self {
        Self { registry }
    }

    pub async fn execute(
        &self,
        request: BackendRequest,
    ) -> Result<serde_json::Value, BackendError> {
        match request {
            BackendRequest::HealthCheck => serialize(HealthCheckResult {
                ok: true,
                backend: "imap-smtp/pop3-smtp",
                version: env!("CARGO_PKG_VERSION"),
            }),
            BackendRequest::ListAccounts => {
                serialize(self.default_account_provider().list_accounts_cap().await?)
            }
            BackendRequest::CreateAccount(draft) => {
                let provider = self.account_provider_for_incoming_protocol(&draft.incoming_protocol);
                serialize(provider.create_account_cap(draft).await?)
            }
            BackendRequest::TestAccountConnection(draft) => {
                let provider = self.account_provider_for_incoming_protocol(&draft.incoming_protocol);
                serialize(provider.test_account_connection_cap(draft).await?)
            }
            BackendRequest::ListFolders { account_id } => {
                let provider = self.folder_provider_for_account(&account_id);
                serialize(provider.list_folders_cap(&account_id).await?)
            }
            BackendRequest::CreateFolder { account_id, name } => {
                let provider = self.folder_provider_for_account(&account_id);
                serialize(provider.create_folder_cap(&account_id, &name).await?)
            }
            BackendRequest::RenameFolder {
                account_id,
                folder_id,
                name,
            } => {
                let provider = self.folder_provider_for_account(&account_id);
                serialize(provider.rename_folder_cap(&account_id, &folder_id, &name).await?)
            }
            BackendRequest::DeleteFolder {
                account_id,
                folder_id,
            } => {
                let provider = self.folder_provider_for_account(&account_id);
                serialize(provider.delete_folder_cap(&account_id, &folder_id).await?)
            }
            BackendRequest::ListMessages {
                account_id,
                folder_id,
            } => {
                let provider = self.message_query_provider_for_account(&account_id);
                serialize(provider.list_messages_cap(&account_id, &folder_id).await?)
            }
            BackendRequest::GetDraft {
                account_id,
                draft_id,
            } => {
                let provider = self.draft_provider_for_account(&account_id);
                serialize(provider.get_draft_cap(&account_id, &draft_id).await?)
            }
            BackendRequest::SearchMessages { account_id, query } => {
                let provider = self.message_query_provider_for_account(&account_id);
                serialize(provider.search_messages_cap(&account_id, &query).await?)
            }
            BackendRequest::ListLabels { account_id } => {
                let provider = self.label_provider_for_account(&account_id);
                serialize(provider.list_labels_cap(&account_id).await?)
            }
            BackendRequest::GetMessage {
                account_id,
                message_id,
            } => {
                let provider = self.message_query_provider_for_account(&account_id);
                serialize(provider.get_message_cap(&account_id, &message_id).await?)
            }
            BackendRequest::AddLabel {
                account_id,
                message_id,
                label,
            } => {
                let provider = self.label_provider_for_account(&account_id);
                serialize(provider.add_label_cap(&account_id, &message_id, &label).await?)
            }
            BackendRequest::RemoveLabel {
                account_id,
                message_id,
                label,
            } => {
                let provider = self.label_provider_for_account(&account_id);
                serialize(provider.remove_label_cap(&account_id, &message_id, &label).await?)
            }
            BackendRequest::RenameLabel {
                account_id,
                label,
                new_label,
            } => {
                let provider = self.label_provider_for_account(&account_id);
                serialize(provider.rename_label_cap(&account_id, &label, &new_label).await?)
            }
            BackendRequest::DeleteLabel { account_id, label } => {
                let provider = self.label_provider_for_account(&account_id);
                serialize(provider.delete_label_cap(&account_id, &label).await?)
            }
            BackendRequest::SaveDraft(draft) => {
                let provider = self.draft_provider_for_account(&draft.account_id);
                serialize(provider.save_draft_cap(draft).await?)
            }
            BackendRequest::SendMessage(draft) => {
                let provider = self.draft_provider_for_account(&draft.account_id);
                serialize(SendMessageResult {
                    ok: true,
                    queued_at: provider.send_message_cap(draft).await?,
                })
            }
            BackendRequest::ToggleStar {
                account_id,
                message_id,
            } => {
                let provider = self.message_mutation_provider_for_account(&account_id);
                serialize(provider.toggle_star_cap(&account_id, &message_id).await?)
            }
            BackendRequest::ToggleRead {
                account_id,
                message_id,
            } => {
                let provider = self.message_mutation_provider_for_account(&account_id);
                serialize(provider.toggle_read_cap(&account_id, &message_id).await?)
            }
            BackendRequest::BatchToggleRead {
                account_id,
                message_ids,
                is_read,
            } => {
                let provider = self.message_mutation_provider_for_account(&account_id);
                serialize(provider.batch_toggle_read_cap(&account_id, &message_ids, is_read).await?)
            }
            BackendRequest::DeleteMessage {
                account_id,
                message_id,
            } => {
                let provider = self.message_mutation_provider_for_account(&account_id);
                serialize(provider.delete_message_cap(&account_id, &message_id).await?)
            }
            BackendRequest::BatchDeleteMessages {
                account_id,
                message_ids,
            } => {
                let provider = self.message_mutation_provider_for_account(&account_id);
                serialize(provider.batch_delete_messages_cap(&account_id, &message_ids).await?)
            }
            BackendRequest::DeleteAccount { account_id } => {
                let provider = self.account_provider_for_account(&account_id);
                serialize(provider.delete_account_cap(&account_id).await?)
            }
            BackendRequest::ArchiveMessage {
                account_id,
                message_id,
            } => {
                let provider = self.message_mutation_provider_for_account(&account_id);
                serialize(provider.archive_message_cap(&account_id, &message_id).await?)
            }
            BackendRequest::RestoreMessage {
                account_id,
                message_id,
            } => {
                let provider = self.message_mutation_provider_for_account(&account_id);
                serialize(provider.restore_message_cap(&account_id, &message_id).await?)
            }
            BackendRequest::MoveMessage {
                account_id,
                message_id,
                folder_id,
            } => {
                let provider = self.message_mutation_provider_for_account(&account_id);
                serialize(provider.move_message_cap(&account_id, &message_id, &folder_id).await?)
            }
            BackendRequest::BatchMoveMessages {
                account_id,
                message_ids,
                folder_id,
            } => {
                let provider = self.message_mutation_provider_for_account(&account_id);
                serialize(provider.batch_move_messages_cap(&account_id, &message_ids, &folder_id).await?)
            }
            BackendRequest::MarkAllRead {
                account_id,
                folder_id,
            } => {
                let provider = self.message_mutation_provider_for_account(&account_id);
                serialize(provider.mark_all_read_cap(&account_id, &folder_id).await?)
            }
            BackendRequest::SyncAccount { account_id } => {
                let provider = self.sync_provider_for_account(&account_id);
                serialize(provider.sync_account_cap(&account_id).await?)
            }
            BackendRequest::GetMailboxBundle { account_id } => {
                let provider = self.sync_provider_for_account(&account_id);
                serialize(provider.get_mailbox_bundle_cap(&account_id).await?)
            }
            BackendRequest::GetAttachmentContent {
                account_id,
                message_id,
                attachment_id,
            } => {
                let provider = self.message_query_provider_for_account(&account_id);
                serialize(
                    provider
                        .get_attachment_content_cap(&account_id, &message_id, &attachment_id)
                        .await?,
                )
            }
            BackendRequest::GetAccountConfig { account_id } => {
                let provider = self.account_provider_for_account(&account_id);
                serialize(provider.get_account_config_cap(&account_id).await?)
            }
            BackendRequest::UpdateAccount { account_id, draft } => {
                let provider = self.account_provider_for_account(&account_id);
                serialize(provider.update_account_cap(&account_id, draft).await?)
            }
            BackendRequest::GetAccountQuota { account_id } => {
                let provider = self.account_provider_for_account(&account_id);
                serialize(provider.get_account_quota_cap(&account_id).await?)
            }
            BackendRequest::ListOAuthProviders => serialize(list_oauth_providers()),
            other => Err(BackendError::internal(format!(
                "Unsupported mail request: {}",
                other.method_name()
            ))),
        }
    }

    fn default_account_provider(&self) -> &'static dyn AccountProvider {
        self.registry.default_account_provider()
    }

    fn account_provider_for_account(&self, account_id: &str) -> &'static dyn AccountProvider {
        self.registry.account_provider_for_account(account_id)
    }

    fn account_provider_for_incoming_protocol(
        &self,
        protocol: &str,
    ) -> &'static dyn AccountProvider {
        self.registry.account_provider_for_incoming_protocol(protocol)
    }

    fn folder_provider_for_account(&self, account_id: &str) -> &'static dyn FolderProvider {
        self.registry.folder_provider_for_account(account_id)
    }

    fn draft_provider_for_account(&self, account_id: &str) -> &'static dyn DraftProvider {
        self.registry.draft_provider_for_account(account_id)
    }

    fn message_query_provider_for_account(
        &self,
        account_id: &str,
    ) -> &'static dyn MessageQueryProvider {
        self.registry.message_query_provider_for_account(account_id)
    }

    fn message_mutation_provider_for_account(
        &self,
        account_id: &str,
    ) -> &'static dyn MessageMutationProvider {
        self.registry.message_mutation_provider_for_account(account_id)
    }

    fn label_provider_for_account(&self, account_id: &str) -> &'static dyn LabelProvider {
        self.registry.label_provider_for_account(account_id)
    }

    fn sync_provider_for_account(&self, account_id: &str) -> &'static dyn SyncProvider {
        self.registry.sync_provider_for_account(account_id)
    }
}
