use crate::protocol::{
    serialize, BackendRequest, HealthCheckResult, SendMessageResult,
};
use crate::oauth::list_oauth_providers;
use crate::provider::imap::IMAP_SMTP_PROVIDER;
use crate::provider::pop3::POP3_SMTP_PROVIDER;
use crate::provider::MailProvider;
use crate::storage::memory;

fn provider_for_account(account_id: &str) -> &'static dyn MailProvider {
    if let Some(state) = memory::get_account_state(account_id) {
        if state.config.incoming_protocol == "pop3" {
            return &POP3_SMTP_PROVIDER;
        }
    }
    &IMAP_SMTP_PROVIDER
}

fn provider_for_draft(protocol: &str) -> &'static dyn MailProvider {
    if protocol == "pop3" {
        &POP3_SMTP_PROVIDER
    } else {
        &IMAP_SMTP_PROVIDER
    }
}

pub async fn handle_with_provider(
    _default_provider: &'static dyn MailProvider,
    request: BackendRequest,
) -> Result<serde_json::Value, crate::protocol::BackendError> {

    match request {
        BackendRequest::HealthCheck => serialize(HealthCheckResult {
            ok: true,
            backend: "imap-smtp/pop3-smtp",
            version: env!("CARGO_PKG_VERSION"),
        }),
        BackendRequest::ListAccounts => serialize(IMAP_SMTP_PROVIDER.list_accounts().await?),
        BackendRequest::CreateAccount(draft) => {
            let provider = provider_for_draft(&draft.incoming_protocol);
            serialize(provider.create_account(draft).await?)
        }
        BackendRequest::TestAccountConnection(draft) => {
            let provider = provider_for_draft(&draft.incoming_protocol);
            serialize(provider.test_account_connection(draft).await?)
        }
        BackendRequest::ListFolders { account_id } => {
            let provider = provider_for_account(&account_id);
            serialize(provider.list_folders(&account_id).await?)
        }
        BackendRequest::ListMessages {
            account_id,
            folder_id,
        } => {
            let provider = provider_for_account(&account_id);
            serialize(provider.list_messages(&account_id, &folder_id).await?)
        }
        BackendRequest::GetMessage {
            account_id,
            message_id,
        } => {
            let provider = provider_for_account(&account_id);
            serialize(provider.get_message(&account_id, &message_id).await?)
        }
        BackendRequest::SaveDraft(draft) => {
            let provider = provider_for_account(&draft.account_id);
            serialize(provider.save_draft(draft).await?)
        }
        BackendRequest::SendMessage(draft) => {
            let provider = provider_for_account(&draft.account_id);
            serialize(SendMessageResult {
                ok: true,
                queued_at: provider.send_message(draft).await?,
            })
        }
        BackendRequest::ToggleStar {
            account_id,
            message_id,
        } => {
            let provider = provider_for_account(&account_id);
            serialize(provider.toggle_star(&account_id, &message_id).await?)
        }
        BackendRequest::ToggleRead {
            account_id,
            message_id,
        } => {
            let provider = provider_for_account(&account_id);
            serialize(provider.toggle_read(&account_id, &message_id).await?)
        }
        BackendRequest::DeleteMessage {
            account_id,
            message_id,
        } => {
            let provider = provider_for_account(&account_id);
            serialize(provider.delete_message(&account_id, &message_id).await?)
        }
        BackendRequest::DeleteAccount { account_id } => {
            let provider = provider_for_account(&account_id);
            serialize(provider.delete_account(&account_id).await?)
        }
        BackendRequest::ArchiveMessage {
            account_id,
            message_id,
        } => {
            let provider = provider_for_account(&account_id);
            serialize(provider.archive_message(&account_id, &message_id).await?)
        }
        BackendRequest::RestoreMessage {
            account_id,
            message_id,
        } => {
            let provider = provider_for_account(&account_id);
            serialize(provider.restore_message(&account_id, &message_id).await?)
        }
        BackendRequest::MoveMessage {
            account_id,
            message_id,
            folder_id,
        } => {
            let provider = provider_for_account(&account_id);
            serialize(provider.move_message(&account_id, &message_id, &folder_id).await?)
        }
        BackendRequest::MarkAllRead {
            account_id,
            folder_id,
        } => {
            let provider = provider_for_account(&account_id);
            serialize(provider.mark_all_read(&account_id, &folder_id).await?)
        }
        BackendRequest::SyncAccount { account_id } => {
            let provider = provider_for_account(&account_id);
            serialize(provider.sync_account(&account_id).await?)
        }
        BackendRequest::GetMailboxBundle { account_id } => {
            let provider = provider_for_account(&account_id);
            serialize(provider.get_mailbox_bundle(&account_id).await?)
        }
        BackendRequest::GetAttachmentContent {
            account_id,
            message_id,
            attachment_id,
        } => {
            let provider = provider_for_account(&account_id);
            serialize(provider.get_attachment_content(&account_id, &message_id, &attachment_id).await?)
        }
        BackendRequest::GetAccountConfig { account_id } => {
            let provider = provider_for_account(&account_id);
            serialize(provider.get_account_config(&account_id).await?)
        }
        BackendRequest::UpdateAccount { account_id, draft } => {
            let provider = provider_for_account(&account_id);
            serialize(provider.update_account(&account_id, draft).await?)
        }
        BackendRequest::ListOAuthProviders => serialize(list_oauth_providers()),
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
