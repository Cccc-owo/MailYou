use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::models::{
    AccountSetupDraft, DraftMessage, MailAccount, MailMessage, MailboxBundle, MailboxFolder,
    SyncStatus,
};

#[derive(Debug, Deserialize)]
#[serde(tag = "method", content = "params", rename_all = "camelCase")]
pub enum BackendRequest {
    HealthCheck,
    ListAccounts,
    CreateAccount(AccountSetupDraft),
    ListFolders { account_id: String },
    ListMessages { account_id: String, folder_id: String },
    GetMessage { account_id: String, message_id: String },
    SaveDraft(DraftMessage),
    SendMessage(DraftMessage),
    ToggleStar { account_id: String, message_id: String },
    ToggleRead { account_id: String, message_id: String },
    DeleteMessage { account_id: String, message_id: String },
    SyncAccount { account_id: String },
    GetMailboxBundle { account_id: String },
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
