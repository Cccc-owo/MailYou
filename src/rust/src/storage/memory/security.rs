use crate::protocol::{BackendError, StorageSecurityStatus};
use crate::storage::persisted;

pub(crate) fn get_security_status() -> Result<StorageSecurityStatus, BackendError> {
    persisted::get_security_status().map_err(|e| BackendError::internal(e.to_string()))
}

pub(crate) fn unlock_storage(password: &str) -> Result<StorageSecurityStatus, BackendError> {
    persisted::unlock_storage(password).map_err(|e| BackendError::validation(e.to_string()))
}

pub(crate) fn set_master_password(
    current_password: Option<&str>,
    new_password: &str,
) -> Result<StorageSecurityStatus, BackendError> {
    if new_password.trim().len() < 8 {
        return Err(BackendError::validation(
            "Master password must be at least 8 characters long",
        ));
    }

    persisted::set_master_password(current_password, new_password)
        .map_err(|e| BackendError::validation(e.to_string()))
}

pub(crate) fn clear_master_password(
    current_password: &str,
) -> Result<StorageSecurityStatus, BackendError> {
    persisted::clear_master_password(current_password)
        .map_err(|e| BackendError::validation(e.to_string()))
}

pub(crate) fn get_storage_dir() -> Result<String, BackendError> {
    let dir = persisted::storage_dir_path().map_err(|e| BackendError::internal(e.to_string()))?;
    dir.to_str()
        .map(|s| s.to_string())
        .ok_or_else(|| BackendError::internal("Non-UTF-8 storage path"))
}
