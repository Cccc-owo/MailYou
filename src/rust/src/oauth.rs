use chrono::{DateTime, Duration, Utc};
use reqwest::Client;
use serde::Deserialize;

use crate::config::oauth::{
    DirectProviderConfig, DEFAULT_PROXY_BASE_URL, DIRECT_PROVIDERS, PROXY_AUTH_TOKEN,
    PROXY_TOKEN_ENV, PROXY_URL_ENV,
};
use crate::models::{
    AccountAuthMode, AccountConfig, OAuthProviderAvailability, OAuthSource, StoredAccountState,
};
use crate::protocol::BackendError;
use crate::storage::memory;

#[derive(Debug, Deserialize)]
struct DirectTokenResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    expires_in: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProxyTokenResponse {
    access_token: String,
    refresh_token: String,
    expires_at: String,
}

#[derive(Debug, Clone)]
pub struct ActiveOAuthToken {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: String,
}

pub fn list_oauth_providers() -> Vec<OAuthProviderAvailability> {
    vec![
        OAuthProviderAvailability {
            id: "gmail".into(),
            label: "Gmail".into(),
            supports_direct: direct_provider_by_id("gmail")
                .is_some_and(direct_provider_is_configured),
            supports_proxy: true,
        },
        OAuthProviderAvailability {
            id: "outlook".into(),
            label: "Outlook".into(),
            supports_direct: direct_provider_by_id("outlook")
                .is_some_and(direct_provider_is_configured),
            supports_proxy: true,
        },
        OAuthProviderAvailability {
            id: "icloud".into(),
            label: "iCloud Mail".into(),
            supports_direct: false,
            supports_proxy: true,
        },
    ]
}

pub async fn ensure_account_access_token(
    state: &StoredAccountState,
) -> Result<Option<ActiveOAuthToken>, BackendError> {
    if !matches!(state.config.auth_mode, AccountAuthMode::Oauth) {
        return Ok(None);
    }

    let token = ensure_config_access_token(&state.config).await?;
    if token.access_token != state.config.access_token
        || token.refresh_token != state.config.refresh_token
        || token.expires_at != state.config.token_expires_at
    {
        memory::store().accounts().update_account_oauth_tokens(
            &state.account.id,
            &token.access_token,
            &token.refresh_token,
            &token.expires_at,
        )?;
    }

    Ok(Some(token))
}

pub async fn ensure_config_access_token(
    config: &AccountConfig,
) -> Result<ActiveOAuthToken, BackendError> {
    if !matches!(config.auth_mode, AccountAuthMode::Oauth) {
        return Err(BackendError::validation(
            "OAuth token requested for a password account",
        ));
    }

    if !config.access_token.trim().is_empty() && !token_expires_soon(&config.token_expires_at) {
        return Ok(ActiveOAuthToken {
            access_token: config.access_token.clone(),
            refresh_token: config.refresh_token.clone(),
            expires_at: config.token_expires_at.clone(),
        });
    }

    refresh_oauth_token(config).await
}

pub fn xoauth2_payload(username: &str, access_token: &str) -> String {
    format!(
        "user={}\x01auth=Bearer {}\x01\x01",
        username.trim(),
        access_token.trim()
    )
}

async fn refresh_oauth_token(config: &AccountConfig) -> Result<ActiveOAuthToken, BackendError> {
    let provider_id = config
        .oauth_provider
        .as_deref()
        .ok_or_else(|| BackendError::validation("OAuth provider is required"))?;
    let refresh_token = config.refresh_token.trim();

    if refresh_token.is_empty() {
        return Err(BackendError::validation(
            "Refresh token is required for OAuth account",
        ));
    }

    match config.oauth_source.clone().unwrap_or(OAuthSource::Direct) {
        OAuthSource::Proxy => refresh_via_proxy(provider_id, refresh_token).await,
        OAuthSource::Direct => refresh_direct(provider_id, refresh_token).await,
    }
}

async fn refresh_via_proxy(
    provider_id: &str,
    refresh_token: &str,
) -> Result<ActiveOAuthToken, BackendError> {
    let proxy_url = proxy_base_url();

    let response = Client::new()
        .post(format!("{proxy_url}/api/refresh"))
        .bearer_auth(proxy_auth_token())
        .json(&serde_json::json!({
            "provider": provider_id,
            "refreshToken": refresh_token,
        }))
        .send()
        .await
        .map_err(|error| BackendError::internal(format!("OAuth proxy refresh failed: {error}")))?;

    if !response.status().is_success() {
        let text = response.text().await.unwrap_or_default();
        return Err(BackendError::validation(format!(
            "OAuth proxy refresh failed: {}",
            extract_proxy_error_message(&text)
        )));
    }

    let payload: ProxyTokenResponse = response.json().await.map_err(|error| {
        BackendError::internal(format!("OAuth proxy response parse failed: {error}"))
    })?;

    Ok(ActiveOAuthToken {
        access_token: payload.access_token,
        refresh_token: if payload.refresh_token.trim().is_empty() {
            refresh_token.to_string()
        } else {
            payload.refresh_token
        },
        expires_at: payload.expires_at,
    })
}

async fn refresh_direct(
    provider_id: &str,
    refresh_token: &str,
) -> Result<ActiveOAuthToken, BackendError> {
    let provider = direct_provider_by_id(provider_id).ok_or_else(|| {
        BackendError::validation(format!(
            "Direct OAuth is not supported for provider '{provider_id}'"
        ))
    })?;

    if !direct_provider_is_configured(provider) {
        return Err(BackendError::validation(format!(
            "Direct OAuth is not configured for provider '{provider_id}'"
        )));
    }

    let client_id = std::env::var(provider.client_id_env).map_err(|_| {
        BackendError::validation(format!("{} is not configured", provider.client_id_env))
    })?;
    let client_secret = std::env::var(provider.client_secret_env).map_err(|_| {
        BackendError::validation(format!("{} is not configured", provider.client_secret_env))
    })?;

    let response = Client::new()
        .post(provider.token_url)
        .form(&[
            ("client_id", client_id.as_str()),
            ("client_secret", client_secret.as_str()),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
        ])
        .send()
        .await
        .map_err(|error| {
            BackendError::internal(format!("OAuth refresh request failed: {error}"))
        })?;

    if !response.status().is_success() {
        let text = response.text().await.unwrap_or_default();
        return Err(BackendError::validation(format!(
            "OAuth refresh failed: {}",
            text.trim()
        )));
    }

    let payload: DirectTokenResponse = response
        .json()
        .await
        .map_err(|error| BackendError::internal(format!("OAuth response parse failed: {error}")))?;

    Ok(ActiveOAuthToken {
        access_token: payload.access_token,
        refresh_token: payload
            .refresh_token
            .unwrap_or_else(|| refresh_token.to_string()),
        expires_at: (Utc::now() + Duration::seconds(payload.expires_in)).to_rfc3339(),
    })
}

fn extract_proxy_error_message(payload: &str) -> String {
    serde_json::from_str::<serde_json::Value>(payload)
        .ok()
        .and_then(|value| {
            value
                .get("error")
                .and_then(|error| {
                    error.as_str().map(ToOwned::to_owned).or_else(|| {
                        error
                            .get("message")
                            .and_then(|message| message.as_str())
                            .map(ToOwned::to_owned)
                    })
                })
                .or_else(|| {
                    value
                        .get("message")
                        .and_then(|message| message.as_str())
                        .map(ToOwned::to_owned)
                })
        })
        .unwrap_or_else(|| payload.trim().to_string())
}

fn direct_provider_by_id(provider_id: &str) -> Option<&'static DirectProviderConfig> {
    DIRECT_PROVIDERS
        .iter()
        .find(|provider| provider.id == provider_id)
}

fn direct_provider_is_configured(provider: &DirectProviderConfig) -> bool {
    env_present(provider.client_id_env) && env_present(provider.client_secret_env)
}

fn env_present(name: &str) -> bool {
    std::env::var(name)
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
}

fn proxy_base_url() -> String {
    std::env::var(PROXY_URL_ENV)
        .ok()
        .map(|value| value.trim().trim_end_matches('/').to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_PROXY_BASE_URL.to_string())
}

fn proxy_auth_token() -> String {
    std::env::var(PROXY_TOKEN_ENV)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| PROXY_AUTH_TOKEN.to_string())
}

fn token_expires_soon(expires_at: &str) -> bool {
    if expires_at.trim().is_empty() {
        return true;
    }

    DateTime::parse_from_rfc3339(expires_at)
        .map(|value| value.with_timezone(&Utc) <= Utc::now() + Duration::seconds(60))
        .unwrap_or(true)
}
