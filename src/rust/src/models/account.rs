use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum AccountStatus {
    #[default]
    Connected,
    Syncing,
    Attention,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum AccountAuthMode {
    #[default]
    Password,
    Oauth,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum OAuthSource {
    #[default]
    Direct,
    Proxy,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct MailIdentity {
    pub id: String,
    pub name: String,
    pub email: String,
    #[serde(default)]
    pub reply_to: Option<String>,
    #[serde(default)]
    pub signature: Option<String>,
    #[serde(default)]
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
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
    #[serde(default)]
    pub identities: Vec<MailIdentity>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
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
    #[serde(default)]
    pub identities: Vec<MailIdentity>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct OAuthProviderAvailability {
    pub id: String,
    pub label: String,
    pub supports_direct: bool,
    pub supports_proxy: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct StoredAccountState {
    pub account: MailAccount,
    pub config: AccountConfig,
}

fn default_incoming_protocol() -> String {
    "imap".into()
}

fn default_auth_mode() -> AccountAuthMode {
    AccountAuthMode::Password
}
