use crate::models::{AttachmentContent, Contact, ContactGroup};
use crate::protocol::BackendError;
use crate::storage::memory::{base64_decode, base64_encode, current_timestamp, lock_state, uuid_short};
use crate::storage::persisted;

pub(crate) fn list_contacts(group_id: Option<&str>) -> Result<Vec<Contact>, BackendError> {
    let state = lock_state();
    let mut out: Vec<Contact> = match group_id {
        Some(gid) => state
            .contacts
            .iter()
            .filter(|c| c.group_id.as_deref() == Some(gid))
            .cloned()
            .collect(),
        None => state.contacts.clone(),
    };
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(out)
}

pub(crate) fn create_contact(mut contact: Contact) -> Result<Contact, BackendError> {
    let mut state = lock_state();
    let now = current_timestamp();
    contact.id = format!("contact-{}", &uuid_short());
    contact.created_at = now.clone();
    contact.updated_at = now;
    state.contacts.push(contact.clone());
    state.persist()?;
    Ok(contact)
}

pub(crate) fn update_contact(contact_id: &str, mut contact: Contact) -> Result<Contact, BackendError> {
    let mut state = lock_state();
    let existing = state
        .contacts
        .iter_mut()
        .find(|c| c.id == contact_id)
        .ok_or_else(|| BackendError::not_found(format!("Contact '{contact_id}' not found")))?;
    contact.id = existing.id.clone();
    contact.created_at = existing.created_at.clone();
    contact.updated_at = current_timestamp();
    *existing = contact.clone();
    state.persist()?;
    Ok(contact)
}

pub(crate) fn delete_contact(contact_id: &str) -> Result<(), BackendError> {
    let mut state = lock_state();
    if state.contacts.iter().any(|c| c.id == contact_id) {
        let _ = persisted::delete_avatar(contact_id);
    }
    state.contacts.retain(|c| c.id != contact_id);
    state.persist()?;
    Ok(())
}

pub(crate) fn search_contacts(query: &str) -> Result<Vec<Contact>, BackendError> {
    let state = lock_state();
    let q = query.to_lowercase();
    let results: Vec<Contact> = state
        .contacts
        .iter()
        .filter(|c| {
            c.name.to_lowercase().contains(&q)
                || c.emails.iter().any(|e| e.to_lowercase().contains(&q))
        })
        .take(20)
        .cloned()
        .collect();
    Ok(results)
}

pub(crate) fn list_contact_groups() -> Result<Vec<ContactGroup>, BackendError> {
    Ok(lock_state().contact_groups.clone())
}

pub(crate) fn create_contact_group(name: String) -> Result<ContactGroup, BackendError> {
    let mut state = lock_state();
    let group = ContactGroup {
        id: format!("cg-{}", &uuid_short()),
        name,
    };
    state.contact_groups.push(group.clone());
    state.persist()?;
    Ok(group)
}

pub(crate) fn update_contact_group(group_id: &str, name: String) -> Result<ContactGroup, BackendError> {
    let mut state = lock_state();
    let group = state
        .contact_groups
        .iter_mut()
        .find(|g| g.id == group_id)
        .ok_or_else(|| BackendError::not_found(format!("Contact group '{group_id}' not found")))?;
    group.name = name;
    let updated = group.clone();
    state.persist()?;
    Ok(updated)
}

pub(crate) fn delete_contact_group(group_id: &str) -> Result<(), BackendError> {
    let mut state = lock_state();
    state.contact_groups.retain(|g| g.id != group_id);
    for contact in state.contacts.iter_mut() {
        if contact.group_id.as_deref() == Some(group_id) {
            contact.group_id = None;
        }
    }
    state.persist()?;
    Ok(())
}

pub(crate) fn upload_contact_avatar(
    contact_id: &str,
    data_base64: &str,
    mime_type: &str,
) -> Result<Contact, BackendError> {
    let decoded = base64_decode(data_base64)
        .map_err(|e| BackendError::validation(format!("Invalid base64: {e}")))?;
    persisted::save_avatar(contact_id, mime_type, &decoded)
        .map_err(|e| BackendError::internal(e.to_string()))?;

    let mut state = lock_state();
    let contact = state
        .contacts
        .iter_mut()
        .find(|c| c.id == contact_id)
        .ok_or_else(|| BackendError::not_found(format!("Contact '{contact_id}' not found")))?;
    contact.avatar_path = Some(contact_id.to_string());
    contact.updated_at = current_timestamp();
    let updated = contact.clone();
    state.persist()?;
    Ok(updated)
}

pub(crate) fn delete_contact_avatar(contact_id: &str) -> Result<Contact, BackendError> {
    let mut state = lock_state();
    let contact = state
        .contacts
        .iter_mut()
        .find(|c| c.id == contact_id)
        .ok_or_else(|| BackendError::not_found(format!("Contact '{contact_id}' not found")))?;
    persisted::delete_avatar(contact_id).map_err(|e| BackendError::internal(e.to_string()))?;

    contact.avatar_path = None;
    contact.updated_at = current_timestamp();
    let updated = contact.clone();
    state.persist()?;
    Ok(updated)
}

pub(crate) fn get_contact_avatar(contact_id: &str) -> Result<Option<AttachmentContent>, BackendError> {
    let Some((mime_type, payload)) =
        persisted::load_avatar(contact_id).map_err(|e| BackendError::internal(e.to_string()))?
    else {
        return Ok(None);
    };

    Ok(Some(AttachmentContent {
        file_name: format!("{contact_id}.webp"),
        mime_type,
        data_base64: base64_encode(&payload),
    }))
}
