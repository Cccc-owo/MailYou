use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AccountStatus {
    Connected,
    Syncing,
    Attention,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AccountAuthMode {
    Password,
    Oauth,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OAuthSource {
    Direct,
    Proxy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MailAccount {
    pub id: String,
    pub name: String,
    pub email: String,
    pub provider: String,
    pub incoming_protocol: String,
    #[serde(default = "default_auth_mode")]
    pub auth_mode: AccountAuthMode,
    #[serde(default)]
    pub oauth_provider: Option<String>,
    #[serde(default)]
    pub oauth_source: Option<OAuthSource>,
    pub color: String,
    pub initials: String,
    pub unread_count: u32,
    pub status: AccountStatus,
    pub last_synced_at: String,
}

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
    #[serde(default = "default_auth_mode")]
    pub auth_mode: AccountAuthMode,
    #[serde(default = "default_incoming_protocol")]
    pub incoming_protocol: String,
    pub incoming_host: String,
    pub incoming_port: u16,
    pub outgoing_host: String,
    pub outgoing_port: u16,
    pub username: String,
    pub password: String,
    pub use_tls: bool,
    #[serde(default)]
    pub oauth_provider: Option<String>,
    #[serde(default)]
    pub oauth_source: Option<OAuthSource>,
    #[serde(default)]
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: String,
    #[serde(default)]
    pub token_expires_at: String,
}

fn default_incoming_protocol() -> String {
    "imap".into()
}

fn default_auth_mode() -> AccountAuthMode {
    AccountAuthMode::Password
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountConfig {
    #[serde(default = "default_auth_mode")]
    pub auth_mode: AccountAuthMode,
    #[serde(default = "default_incoming_protocol")]
    pub incoming_protocol: String,
    pub incoming_host: String,
    pub incoming_port: u16,
    pub outgoing_host: String,
    pub outgoing_port: u16,
    pub username: String,
    pub password: String,
    pub use_tls: bool,
    #[serde(default)]
    pub oauth_provider: Option<String>,
    #[serde(default)]
    pub oauth_source: Option<OAuthSource>,
    #[serde(default)]
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: String,
    #[serde(default)]
    pub token_expires_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthProviderAvailability {
    pub id: String,
    pub label: String,
    pub supports_direct: bool,
    pub supports_proxy: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredAccountState {
    pub account: MailAccount,
    pub config: AccountConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftMessage {
    pub id: String,
    pub account_id: String,
    pub to: String,
    pub cc: String,
    pub bcc: String,
    pub subject: String,
    pub body: String,
    pub in_reply_to_message_id: Option<String>,
    pub forward_from_message_id: Option<String>,
    #[serde(default)]
    pub attachments: Vec<DraftAttachment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftAttachment {
    pub file_name: String,
    pub mime_type: String,
    pub data_base64: String,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Contact {
    pub id: String,
    pub name: String,
    #[serde(default, deserialize_with = "deserialize_emails", alias = "email")]
    pub emails: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_phones", alias = "phone")]
    pub phones: Vec<String>,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub group_id: Option<String>,
    #[serde(default)]
    pub avatar_path: Option<String>,
    #[serde(default)]
    pub source_account_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Backward-compat deserializer: accepts both `"email": "a@b"` (old) and `"emails": ["a@b"]` (new).
fn deserialize_emails<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringOrVec {
        Single(String),
        Multiple(Vec<String>),
    }

    match StringOrVec::deserialize(deserializer)? {
        StringOrVec::Single(s) => {
            if s.is_empty() {
                Ok(Vec::new())
            } else {
                Ok(vec![s])
            }
        }
        StringOrVec::Multiple(v) => Ok(v),
    }
}

/// Backward-compat deserializer: accepts `"phone": "123"` (old) and `"phones": ["123"]` (new), plus `null`.
fn deserialize_phones<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum PhoneCompat {
        Null,
        Single(String),
        Multiple(Vec<String>),
    }

    match PhoneCompat::deserialize(deserializer)? {
        PhoneCompat::Null => Ok(Vec::new()),
        PhoneCompat::Single(s) => {
            if s.is_empty() {
                Ok(Vec::new())
            } else {
                Ok(vec![s])
            }
        }
        PhoneCompat::Multiple(v) => Ok(v),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContactGroup {
    pub id: String,
    pub name: String,
}
