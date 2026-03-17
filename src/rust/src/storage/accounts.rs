use crate::models::{AccountConfig, AccountSetupDraft};

pub fn config_from_draft(draft: &AccountSetupDraft) -> AccountConfig {
    AccountConfig {
        auth_mode: draft.auth_mode.clone(),
        incoming_protocol: draft.incoming_protocol.clone(),
        incoming_host: draft.incoming_host.clone(),
        incoming_port: draft.incoming_port,
        outgoing_host: draft.outgoing_host.clone(),
        outgoing_port: draft.outgoing_port,
        username: draft.username.clone(),
        password: draft.password.clone(),
        use_tls: draft.use_tls,
        oauth_provider: draft.oauth_provider.clone(),
        oauth_source: draft.oauth_source.clone(),
        access_token: draft.access_token.clone(),
        refresh_token: draft.refresh_token.clone(),
        token_expires_at: draft.token_expires_at.clone(),
    }
}
