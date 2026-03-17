use crate::models::{AccountAuthMode, DraftMessage, StoredAccountState};
use crate::oauth::ensure_account_access_token;
use crate::protocol::BackendError;

pub(super) async fn smtp_send(
    state: &StoredAccountState,
    draft: &DraftMessage,
) -> Result<Vec<u8>, BackendError> {
    use lettre::transport::smtp::authentication::{Credentials, Mechanism};
    use lettre::{AsyncSmtpTransport, AsyncTransport, Tokio1Executor};

    let email = super::draft_ops::build_rfc822_message(state, draft)?;
    let raw_email = email.formatted();
    let host = state.config.outgoing_host.trim();
    let port = state.config.outgoing_port;
    let (creds, mechanisms) = match state.config.auth_mode {
        AccountAuthMode::Password => (
            Credentials::new(state.config.username.clone(), state.config.password.clone()),
            None,
        ),
        AccountAuthMode::Oauth => {
            let token = ensure_account_access_token(state)
                .await?
                .ok_or_else(|| BackendError::validation("OAuth token is missing"))?;
            (
                Credentials::new(state.config.username.clone(), token.access_token),
                Some(vec![Mechanism::Xoauth2]),
            )
        }
    };

    let transport_builder = if state.config.use_tls {
        AsyncSmtpTransport::<Tokio1Executor>::relay(host)
            .map_err(|e| BackendError::internal(format!("SMTP relay error: {e}")))?
            .port(port)
    } else {
        AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(host).port(port)
    };
    let transport_builder = transport_builder.credentials(creds);
    let transport = if let Some(mechanisms) = mechanisms {
        transport_builder.authentication(mechanisms).build()
    } else {
        transport_builder.build()
    };

    transport
        .send(email)
        .await
        .map_err(|e| BackendError::internal(format!("SMTP send failed: {e}")))?;

    Ok(raw_email)
}
