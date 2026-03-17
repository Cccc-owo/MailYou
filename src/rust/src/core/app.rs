use crate::core::local_handlers::{handle_local_request, is_local_request};
use crate::core::mail_handlers::handle_mail_request;
use crate::protocol::{BackendError, BackendRequest};
use crate::provider::registry::ProviderRegistry;

pub async fn handle_request(
    registry: &ProviderRegistry,
    request: BackendRequest,
) -> Result<serde_json::Value, BackendError> {
    if is_local_request(&request) {
        handle_local_request(request)
    } else {
        handle_mail_request(registry, request).await
    }
}
