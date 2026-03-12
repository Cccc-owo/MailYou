use std::collections::HashMap;

use crate::models::SyncStatus;

pub fn seeded_sync_statuses() -> HashMap<String, SyncStatus> {
    HashMap::from([
        (
            "acc-work".into(),
            SyncStatus {
                account_id: "acc-work".into(),
                state: "syncing".into(),
                message: "Last sync completed 4 minutes ago".into(),
                updated_at: "2026-03-12T09:41:00.000Z".into(),
            },
        ),
        (
            "acc-personal".into(),
            SyncStatus {
                account_id: "acc-personal".into(),
                state: "idle".into(),
                message: "Mailbox is up to date".into(),
                updated_at: "2026-03-12T09:27:00.000Z".into(),
            },
        ),
    ])
}

pub fn initial_sync_status(account_id: &str, updated_at: &str) -> SyncStatus {
    SyncStatus {
        account_id: account_id.into(),
        state: "idle".into(),
        message: "Mailbox is up to date".into(),
        updated_at: updated_at.into(),
    }
}
