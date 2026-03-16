use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::models::{
    AccountSetupDraft, AttachmentContent, Contact, ContactGroup, DraftMessage, MailAccount,
    MailMessage, MailboxBundle, MailboxFolder, SyncStatus,
    OAuthProviderAvailability,
};

#[derive(Debug, Deserialize)]
#[serde(tag = "method", content = "params", rename_all = "camelCase")]
pub enum BackendRequest {
    HealthCheck,
    ListAccounts,
    CreateAccount(AccountSetupDraft),
    TestAccountConnection(AccountSetupDraft),
    #[serde(rename_all = "camelCase")]
    ListFolders { account_id: String },
    #[serde(rename_all = "camelCase")]
    ListMessages { account_id: String, folder_id: String },
    #[serde(rename_all = "camelCase")]
    GetMessage { account_id: String, message_id: String },
    SaveDraft(DraftMessage),
    SendMessage(DraftMessage),
    #[serde(rename_all = "camelCase")]
    ToggleStar { account_id: String, message_id: String },
    #[serde(rename_all = "camelCase")]
    ToggleRead { account_id: String, message_id: String },
    #[serde(rename_all = "camelCase")]
    DeleteMessage { account_id: String, message_id: String },
    #[serde(rename_all = "camelCase")]
    DeleteAccount { account_id: String },
    #[serde(rename_all = "camelCase")]
    ArchiveMessage { account_id: String, message_id: String },
    #[serde(rename_all = "camelCase")]
    RestoreMessage { account_id: String, message_id: String },
    #[serde(rename_all = "camelCase")]
    MoveMessage { account_id: String, message_id: String, folder_id: String },
    #[serde(rename_all = "camelCase")]
    MarkAllRead { account_id: String, folder_id: String },
    #[serde(rename_all = "camelCase")]
    SyncAccount { account_id: String },
    #[serde(rename_all = "camelCase")]
    GetMailboxBundle { account_id: String },
    #[serde(rename_all = "camelCase")]
    GetAttachmentContent { account_id: String, message_id: String, attachment_id: String },
    #[serde(rename_all = "camelCase")]
    GetAccountConfig { account_id: String },
    #[serde(rename_all = "camelCase")]
    UpdateAccount { account_id: String, draft: AccountSetupDraft },
    ListOAuthProviders,
    // -- Contacts (local-only, bypass MailProvider) --
    #[serde(rename_all = "camelCase")]
    ListContacts { group_id: Option<String> },
    CreateContact(Contact),
    #[serde(rename_all = "camelCase")]
    UpdateContact { contact_id: String, contact: Contact },
    #[serde(rename_all = "camelCase")]
    DeleteContact { contact_id: String },
    #[serde(rename_all = "camelCase")]
    SearchContacts { query: String },
    ListContactGroups,
    #[serde(rename_all = "camelCase")]
    CreateContactGroup { name: String },
    #[serde(rename_all = "camelCase")]
    UpdateContactGroup { group_id: String, name: String },
    #[serde(rename_all = "camelCase")]
    DeleteContactGroup { group_id: String },
    #[serde(rename_all = "camelCase")]
    UploadContactAvatar { contact_id: String, data_base64: String, mime_type: String },
    #[serde(rename_all = "camelCase")]
    DeleteContactAvatar { contact_id: String },
    GetStorageDir,
}

impl BackendRequest {
    pub fn method_name(&self) -> &'static str {
        match self {
            Self::HealthCheck => "healthCheck",
            Self::ListAccounts => "listAccounts",
            Self::CreateAccount(_) => "createAccount",
            Self::TestAccountConnection(_) => "testAccountConnection",
            Self::ListFolders { .. } => "listFolders",
            Self::ListMessages { .. } => "listMessages",
            Self::GetMessage { .. } => "getMessage",
            Self::SaveDraft(_) => "saveDraft",
            Self::SendMessage(_) => "sendMessage",
            Self::ToggleStar { .. } => "toggleStar",
            Self::ToggleRead { .. } => "toggleRead",
            Self::DeleteMessage { .. } => "deleteMessage",
            Self::DeleteAccount { .. } => "deleteAccount",
            Self::ArchiveMessage { .. } => "archiveMessage",
            Self::RestoreMessage { .. } => "restoreMessage",
            Self::MoveMessage { .. } => "moveMessage",
            Self::MarkAllRead { .. } => "markAllRead",
            Self::SyncAccount { .. } => "syncAccount",
            Self::GetMailboxBundle { .. } => "getMailboxBundle",
            Self::GetAttachmentContent { .. } => "getAttachmentContent",
            Self::GetAccountConfig { .. } => "getAccountConfig",
            Self::UpdateAccount { .. } => "updateAccount",
            Self::ListOAuthProviders => "listOAuthProviders",
            Self::ListContacts { .. } => "listContacts",
            Self::CreateContact(_) => "createContact",
            Self::UpdateContact { .. } => "updateContact",
            Self::DeleteContact { .. } => "deleteContact",
            Self::SearchContacts { .. } => "searchContacts",
            Self::ListContactGroups => "listContactGroups",
            Self::CreateContactGroup { .. } => "createContactGroup",
            Self::UpdateContactGroup { .. } => "updateContactGroup",
            Self::DeleteContactGroup { .. } => "deleteContactGroup",
            Self::UploadContactAvatar { .. } => "uploadContactAvatar",
            Self::DeleteContactAvatar { .. } => "deleteContactAvatar",
            Self::GetStorageDir => "getStorageDir",
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct BackendRequestEnvelope {
    pub id: u64,
    #[serde(flatten)]
    pub request: BackendRequest,
}

#[derive(Debug, Serialize)]
pub struct BackendError {
    pub code: String,
    pub message: String,
}

impl BackendError {
    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            code: "internal_error".into(),
            message: message.into(),
        }
    }

    pub fn validation(message: impl Into<String>) -> Self {
        Self {
            code: "validation_error".into(),
            message: message.into(),
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            code: "not_found".into(),
            message: message.into(),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct BackendSuccessResponse {
    pub id: u64,
    pub ok: bool,
    pub result: Value,
}

#[derive(Debug, Serialize)]
pub struct BackendErrorResponse {
    pub id: u64,
    pub ok: bool,
    pub error: BackendError,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum BackendResponse {
    Success(BackendSuccessResponse),
    Error(BackendErrorResponse),
}

impl BackendResponse {
    pub fn success(id: u64, result: Value) -> Self {
        Self::Success(BackendSuccessResponse {
            id,
            ok: true,
            result,
        })
    }

    pub fn error(id: u64, error: BackendError) -> Self {
        Self::Error(BackendErrorResponse {
            id,
            ok: false,
            error,
        })
    }
}

pub fn serialize<T: Serialize>(value: T) -> Result<Value, BackendError> {
    serde_json::to_value(value).map_err(|error| BackendError::internal(error.to_string()))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthCheckResult {
    pub ok: bool,
    pub backend: &'static str,
    pub version: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SendMessageResult {
    pub ok: bool,
    pub queued_at: String,
}

#[allow(dead_code)]
pub type ListAccountsResult = Vec<MailAccount>;
#[allow(dead_code)]
pub type TestAccountConnectionResult = SyncStatus;
#[allow(dead_code)]
pub type ListFoldersResult = Vec<MailboxFolder>;
#[allow(dead_code)]
pub type ListMessagesResult = Vec<MailMessage>;
#[allow(dead_code)]
pub type GetMessageResult = Option<MailMessage>;
#[allow(dead_code)]
pub type SaveDraftResult = DraftMessage;
#[allow(dead_code)]
pub type SendMessageResponse = SendMessageResult;
#[allow(dead_code)]
pub type ToggleMessageResult = Option<MailMessage>;
#[allow(dead_code)]
pub type DeleteMessageResult = ();
#[allow(dead_code)]
pub type SyncAccountResult = SyncStatus;
#[allow(dead_code)]
pub type GetMailboxBundleResult = MailboxBundle;
#[allow(dead_code)]
pub type GetAttachmentContentResult = AttachmentContent;
#[allow(dead_code)]
pub type ListOAuthProvidersResult = Vec<OAuthProviderAvailability>;
#[allow(dead_code)]
pub type ListContactsResult = Vec<Contact>;
#[allow(dead_code)]
pub type SearchContactsResult = Vec<Contact>;
#[allow(dead_code)]
pub type ListContactGroupsResult = Vec<ContactGroup>;
