use crate::protocol::{
    serialize, BackendRequest, HealthCheckResult, SendMessageResult,
};
use crate::provider::MailProvider;
use crate::storage::memory;

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
        BackendRequest::GetAccountConfig { account_id } => serialize(provider.get_account_config(&account_id).await?),
        BackendRequest::UpdateAccount { account_id, draft } => serialize(provider.update_account(&account_id, draft).await?),
        // -- Contacts (local-only, bypass MailProvider) --
        BackendRequest::ListContacts { group_id } => serialize(memory::list_contacts(group_id.as_deref())?),
        BackendRequest::CreateContact(contact) => serialize(memory::create_contact(contact)?),
        BackendRequest::UpdateContact { contact_id, contact } => serialize(memory::update_contact(&contact_id, contact)?),
        BackendRequest::DeleteContact { contact_id } => serialize(memory::delete_contact(&contact_id)?),
        BackendRequest::SearchContacts { query } => serialize(memory::search_contacts(&query)?),
        BackendRequest::ListContactGroups => serialize(memory::list_contact_groups()?),
        BackendRequest::CreateContactGroup { name } => serialize(memory::create_contact_group(name)?),
        BackendRequest::UpdateContactGroup { group_id, name } => serialize(memory::update_contact_group(&group_id, name)?),
        BackendRequest::DeleteContactGroup { group_id } => serialize(memory::delete_contact_group(&group_id)?),
        BackendRequest::UploadContactAvatar { contact_id, data_base64, mime_type } => serialize(memory::upload_contact_avatar(&contact_id, &data_base64, &mime_type)?),
        BackendRequest::DeleteContactAvatar { contact_id } => serialize(memory::delete_contact_avatar(&contact_id)?),
        BackendRequest::GetStorageDir => serialize(memory::get_storage_dir()?),
    }
}
