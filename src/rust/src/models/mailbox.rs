use serde::{Deserialize, Serialize};

use crate::models::sync::SyncStatus;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MailFolderKind {
    Inbox,
    Sent,
    Drafts,
    Trash,
    Junk,
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
    #[serde(default)]
    pub imap_name: Option<String>,
    #[serde(default)]
    pub imap_uid_validity: Option<u32>,
    #[serde(default)]
    pub imap_uid_next: Option<u32>,
    #[serde(default)]
    pub imap_highest_modseq: Option<u64>,
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
    #[serde(default)]
    pub imap_uid: Option<u32>,
    #[serde(default)]
    pub previous_folder_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MailLabel {
    pub name: String,
    pub count: u32,
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
pub struct AttachmentContent {
    pub file_name: String,
    pub mime_type: String,
    pub data_base64: String,
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
