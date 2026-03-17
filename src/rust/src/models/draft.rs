use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftMessage {
    pub id: String,
    pub account_id: String,
    #[serde(default)]
    pub selected_identity_id: Option<String>,
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
