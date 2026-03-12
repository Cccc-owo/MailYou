use crate::core::context::AppContext;
use crate::protocol::{
    serialize, BackendRequest, HealthCheckResult, SendMessageResult,
};

pub fn handle(
    context: &AppContext,
    request: BackendRequest,
) -> Result<serde_json::Value, crate::protocol::BackendError> {
    let provider = context.provider();

    match request {
        BackendRequest::HealthCheck => serialize(HealthCheckResult {
            ok: true,
            backend: provider.backend_name(),
            version: env!("CARGO_PKG_VERSION"),
        }),
        BackendRequest::ListAccounts => serialize(provider.list_accounts()?),
        BackendRequest::CreateAccount(draft) => serialize(provider.create_account(draft)?),
        BackendRequest::TestAccountConnection(draft) => serialize(provider.test_account_connection(draft)?),
        BackendRequest::ListFolders { account_id } => serialize(provider.list_folders(&account_id)?),
        BackendRequest::ListMessages {
            account_id,
            folder_id,
        } => serialize(provider.list_messages(&account_id, &folder_id)?),
        BackendRequest::GetMessage {
            account_id,
            message_id,
        } => serialize(provider.get_message(&account_id, &message_id)?),
        BackendRequest::SaveDraft(draft) => serialize(provider.save_draft(draft)?),
        BackendRequest::SendMessage(draft) => serialize(SendMessageResult {
            ok: true,
            queued_at: provider.send_message(draft)?,
        }),
        BackendRequest::ToggleStar {
            account_id,
            message_id,
        } => serialize(provider.toggle_star(&account_id, &message_id)?),
        BackendRequest::ToggleRead {
            account_id,
            message_id,
        } => serialize(provider.toggle_read(&account_id, &message_id)?),
        BackendRequest::DeleteMessage {
            account_id,
            message_id,
        } => serialize(provider.delete_message(&account_id, &message_id)?),
        BackendRequest::ArchiveMessage {
            account_id,
            message_id,
        } => serialize(provider.archive_message(&account_id, &message_id)?),
        BackendRequest::RestoreMessage {
            account_id,
            message_id,
        } => serialize(provider.restore_message(&account_id, &message_id)?),
        BackendRequest::MoveMessage {
            account_id,
            message_id,
            folder_id,
        } => serialize(provider.move_message(&account_id, &message_id, &folder_id)?),
        BackendRequest::SyncAccount { account_id } => serialize(provider.sync_account(&account_id)?),
        BackendRequest::GetMailboxBundle { account_id } => serialize(provider.get_mailbox_bundle(&account_id)?),
    }
}
