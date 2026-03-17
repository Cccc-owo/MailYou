use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MailIdentity {
    pub id: String,
    pub name: String,
    pub email: String,
    pub reply_to: Option<String>,
    pub signature: Option<String>,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AccountStatus {
    Connected,
    Syncing,
    Attention,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MailAccount {
    pub id: String,
    pub name: String,
    pub email: String,
    pub provider: String,
    pub color: String,
    pub initials: String,
    pub unread_count: u32,
    pub status: AccountStatus,
    pub last_synced_at: String,
    pub identities: Vec<MailIdentity>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MailFolderKind {
    Inbox,
    Sent,
    Drafts,
    Trash,
    Starred,
    Archive,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MailboxFolder {
    pub id: String,
    pub account_id: String,
    pub name: String,
    pub kind: MailFolderKind,
    pub unread_count: u32,
    pub total_count: u32,
    pub icon: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentMeta {
    pub id: String,
    pub file_name: String,
    pub mime_type: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MailMessage {
    pub id: String,
    pub account_id: String,
    pub folder_id: String,
    pub thread_id: String,
    pub subject: String,
    pub preview: String,
    pub body: String,
    pub from: String,
    pub from_email: String,
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub sent_at: String,
    pub received_at: String,
    pub is_read: bool,
    pub is_starred: bool,
    pub has_attachments: bool,
    pub attachments: Vec<AttachmentMeta>,
    pub labels: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MailThread {
    pub id: String,
    pub account_id: String,
    pub subject: String,
    pub message_ids: Vec<String>,
    pub last_message_at: String,
    pub unread_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountSetupDraft {
    pub display_name: String,
    pub email: String,
    pub provider: String,
    pub incoming_host: String,
    pub incoming_port: u16,
    pub outgoing_host: String,
    pub outgoing_port: u16,
    pub username: String,
    pub password: String,
    pub use_tls: bool,
    pub identities: Vec<MailIdentity>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftMessage {
    pub id: String,
    pub account_id: String,
    pub selected_identity_id: Option<String>,
    pub to: String,
    pub cc: String,
    pub bcc: String,
    pub subject: String,
    pub body: String,
    pub in_reply_to_message_id: Option<String>,
    pub forward_from_message_id: Option<String>,
}

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
pub struct MailboxBundle {
    pub account_id: String,
    pub folders: Vec<MailboxFolder>,
    pub messages: Vec<MailMessage>,
    pub threads: Vec<MailThread>,
    pub sync_status: SyncStatus,
}
