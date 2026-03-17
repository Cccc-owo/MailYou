use crate::protocol::{serialize, BackendError, BackendRequest};
use crate::storage::memory;

pub fn is_local_request(request: &BackendRequest) -> bool {
    matches!(
        request,
        BackendRequest::ListContacts { .. }
            | BackendRequest::CreateContact(_)
            | BackendRequest::UpdateContact { .. }
            | BackendRequest::DeleteContact { .. }
            | BackendRequest::SearchContacts { .. }
            | BackendRequest::ListContactGroups
            | BackendRequest::CreateContactGroup { .. }
            | BackendRequest::UpdateContactGroup { .. }
            | BackendRequest::DeleteContactGroup { .. }
            | BackendRequest::UploadContactAvatar { .. }
            | BackendRequest::DeleteContactAvatar { .. }
            | BackendRequest::GetContactAvatar { .. }
            | BackendRequest::GetSecurityStatus
            | BackendRequest::UnlockStorage { .. }
            | BackendRequest::SetMasterPassword { .. }
            | BackendRequest::ClearMasterPassword { .. }
            | BackendRequest::GetStorageDir
    )
}

pub fn handle_local_request(request: BackendRequest) -> Result<serde_json::Value, BackendError> {
    let store = memory::store();
    let contacts = store.contacts();
    let security = store.security();

    match request {
        BackendRequest::ListContacts { group_id } => {
            serialize(contacts.list_contacts(group_id.as_deref())?)
        }
        BackendRequest::CreateContact(contact) => serialize(contacts.create_contact(contact)?),
        BackendRequest::UpdateContact {
            contact_id,
            contact,
        } => serialize(contacts.update_contact(&contact_id, contact)?),
        BackendRequest::DeleteContact { contact_id } => {
            serialize(contacts.delete_contact(&contact_id)?)
        }
        BackendRequest::SearchContacts { query } => serialize(contacts.search_contacts(&query)?),
        BackendRequest::ListContactGroups => serialize(contacts.list_contact_groups()?),
        BackendRequest::CreateContactGroup { name } => {
            serialize(contacts.create_contact_group(name)?)
        }
        BackendRequest::UpdateContactGroup { group_id, name } => {
            serialize(contacts.update_contact_group(&group_id, name)?)
        }
        BackendRequest::DeleteContactGroup { group_id } => {
            serialize(contacts.delete_contact_group(&group_id)?)
        }
        BackendRequest::UploadContactAvatar {
            contact_id,
            data_base64,
            mime_type,
        } => serialize(contacts.upload_contact_avatar(
            &contact_id,
            &data_base64,
            &mime_type,
        )?),
        BackendRequest::DeleteContactAvatar { contact_id } => {
            serialize(contacts.delete_contact_avatar(&contact_id)?)
        }
        BackendRequest::GetContactAvatar { contact_id } => {
            serialize(contacts.get_contact_avatar(&contact_id)?)
        }
        BackendRequest::GetSecurityStatus => serialize(security.get_security_status()?),
        BackendRequest::UnlockStorage { password } => serialize(security.unlock_storage(&password)?),
        BackendRequest::SetMasterPassword {
            current_password,
            new_password,
        } => serialize(security.set_master_password(
            current_password.as_deref(),
            &new_password,
        )?),
        BackendRequest::ClearMasterPassword { current_password } => {
            serialize(security.clear_master_password(&current_password)?)
        }
        BackendRequest::GetStorageDir => serialize(security.get_storage_dir()?),
        other => Err(BackendError::internal(format!(
            "Unsupported local request: {}",
            other.method_name()
        ))),
    }
}
