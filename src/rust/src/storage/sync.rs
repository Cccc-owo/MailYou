use crate::models::SyncStatus;

pub fn initial_sync_status(account_id: &str, updated_at: &str) -> SyncStatus {
    SyncStatus {
        account_id: account_id.into(),
        state: "idle".into(),
        message: "Mailbox is up to date".into(),
        updated_at: updated_at.into(),
    }
}
