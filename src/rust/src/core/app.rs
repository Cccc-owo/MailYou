use crate::protocol::{
    serialize, BackendRequest, HealthCheckResult, SendMessageResult,
};
use crate::provider::MailProvider;

pub async fn handle_with_provider(
    provider: &'static dyn MailProvider,
    request: BackendRequest,
) -> Result<serde_json::Value, crate::protocol::BackendError> {

    match request {
        BackendRequest::HealthCheck => serialize(HealthCheckResult {
            ok: true,
            backend: provider.backend_name(),
            version: env!("CARGO_PKG_VERSION"),
        }),
        BackendRequest::ListAccounts => serialize(provider.list_accounts().await?),
        BackendRequest::CreateAccount(draft) => serialize(provider.create_account(draft).await?),
        BackendRequest::TestAccountConnection(draft) => serialize(provider.test_account_connection(draft).await?),
        BackendRequest::ListFolders { account_id } => serialize(provider.list_folders(&account_id).await?),
        BackendRequest::ListMessages {
            account_id,
            folder_id,
        } => serialize(provider.list_messages(&account_id, &folder_id).await?),
        BackendRequest::GetMessage {
            account_id,
            message_id,
        } => serialize(provider.get_message(&account_id, &message_id).await?),
        BackendRequest::SaveDraft(draft) => serialize(provider.save_draft(draft).await?),
        BackendRequest::SendMessage(draft) => serialize(SendMessageResult {
            ok: true,
            queued_at: provider.send_message(draft).await?,
        }),
        BackendRequest::ToggleStar {
            account_id,
            message_id,
        } => serialize(provider.toggle_star(&account_id, &message_id).await?),
        BackendRequest::ToggleRead {
            account_id,
            message_id,
        } => serialize(provider.toggle_read(&account_id, &message_id).await?),
        BackendRequest::DeleteMessage {
            account_id,
            message_id,
        } => serialize(provider.delete_message(&account_id, &message_id).await?),
        BackendRequest::DeleteAccount { account_id } => serialize(provider.delete_account(&account_id).await?),
        BackendRequest::ArchiveMessage {
            account_id,
            message_id,
        } => serialize(provider.archive_message(&account_id, &message_id).await?),
        BackendRequest::RestoreMessage {
            account_id,
            message_id,
        } => serialize(provider.restore_message(&account_id, &message_id).await?),
        BackendRequest::MoveMessage {
            account_id,
            message_id,
            folder_id,
        } => serialize(provider.move_message(&account_id, &message_id, &folder_id).await?),
        BackendRequest::MarkAllRead {
            account_id,
            folder_id,
        } => serialize(provider.mark_all_read(&account_id, &folder_id).await?),
        BackendRequest::SyncAccount { account_id } => serialize(provider.sync_account(&account_id).await?),
        BackendRequest::GetMailboxBundle { account_id } => serialize(provider.get_mailbox_bundle(&account_id).await?),
        BackendRequest::GetAttachmentContent {
            account_id,
            message_id,
            attachment_id,
        } => serialize(provider.get_attachment_content(&account_id, &message_id, &attachment_id).await?),
    }
}
