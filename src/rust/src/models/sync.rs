use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatus {
    pub account_id: String,
    pub state: String,
    pub message: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountQuota {
    pub account_id: String,
    pub quota_root: Option<String>,
    pub storage_used_kb: Option<u64>,
    pub storage_limit_kb: Option<u64>,
    pub usage_percent: Option<u64>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountUnreadMessageSummary {
    pub id: String,
    pub subject: String,
    pub from: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountUnreadSnapshot {
    pub account_id: String,
    pub unread_messages: Vec<AccountUnreadMessageSummary>,
    pub updated_at: String,
}
