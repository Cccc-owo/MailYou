use crate::core::mail_service::MailService;
use crate::protocol::{BackendError, BackendRequest};
use crate::provider::registry::ProviderRegistry;

pub async fn handle_mail_request(
    registry: &ProviderRegistry,
    request: BackendRequest,
) -> Result<serde_json::Value, BackendError> {
    MailService::new(registry).execute(request).await
}
