use std::time::Instant;

use futures::TryStreamExt;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_native_tls::TlsStream;

use crate::models::{AccountAuthMode, AccountConfig, AccountSetupDraft, StoredAccountState};
use crate::oauth::{ensure_account_access_token, ensure_config_access_token, xoauth2_payload};
use crate::protocol::BackendError;
use crate::storage::memory;

pub(super) async fn imap_login_test(draft: &AccountSetupDraft) -> Result<(), BackendError> {
    let host = draft.incoming_host.trim();
    let port = draft.incoming_port;

    eprintln!("[imap] tcp connecting to {host}:{port}...");
    let tcp = imap_tcp_connect(host, port)
        .await
        .map_err(|e| BackendError::validation(e.message))?;
    eprintln!("[imap] tcp connected, tls={}...", draft.use_tls);

    if draft.use_tls {
        let client = match imap_tls_connect(host, tcp).await {
            Ok(tls_stream) => {
                let mut client = async_imap::Client::new(tls_stream);
                imap_read_greeting(&mut client)
                    .await
                    .map_err(|e| BackendError::validation(e.message))?;
                client
            }
            Err(tls_error) => {
                eprintln!(
                    "[imap] implicit TLS failed, trying STARTTLS fallback: {}",
                    tls_error.message
                );
                imap_upgrade_starttls(
                    host,
                    imap_tcp_connect(host, port)
                        .await
                        .map_err(|e| BackendError::validation(e.message))?,
                )
                .await
                .map_err(|starttls_error| {
                    BackendError::validation(format!(
                        "{}; STARTTLS fallback failed: {}",
                        tls_error.message, starttls_error.message
                    ))
                })?
            }
        };
        eprintln!("[imap] logging in as {}...", draft.username.trim());
        let mut session = imap_authenticate_client(client, draft).await?;
        let _ = session.logout().await;
    } else {
        let mut client = async_imap::Client::new(tcp);
        imap_read_greeting(&mut client)
            .await
            .map_err(|e| BackendError::validation(e.message))?;
        eprintln!("[imap] logging in as {}...", draft.username.trim());
        let mut session = imap_authenticate_client(client, draft).await?;
        let _ = session.logout().await;
    }

    Ok(())
}

pub(super) async fn imap_connect(
    state: &StoredAccountState,
) -> Result<super::ImapAnySession, BackendError> {
    let host = state.config.incoming_host.trim();
    let port = state.config.incoming_port;
    let use_tls = state.config.use_tls;

    eprintln!("[imap] connecting to {host}:{port} (tls={use_tls})...");
    let start = Instant::now();
    let tcp = imap_tcp_connect(host, port).await?;

    let stream = if use_tls {
        match imap_tls_connect(host, tcp).await {
            Ok(tls_stream) => {
                let mut client = async_imap::Client::new(tls_stream);
                imap_read_greeting(&mut client).await?;
                super::ImapStream::Tls(client.into_inner())
            }
            Err(tls_error) => {
                eprintln!(
                    "[imap] implicit TLS failed, trying STARTTLS fallback: {}",
                    tls_error.message
                );
                let client =
                    imap_upgrade_starttls(host, imap_tcp_connect(host, port).await?).await?;
                super::ImapStream::Tls(client.into_inner())
            }
        }
    } else {
        let mut client = async_imap::Client::new(tcp);
        imap_read_greeting(&mut client).await?;
        super::ImapStream::Plain(client.into_inner())
    };

    let client = async_imap::Client::new(stream);
    let session = match state.config.auth_mode {
        AccountAuthMode::Password => client
            .login(state.config.username.trim(), state.config.password.trim())
            .await
            .map_err(|e| BackendError::internal(format!("IMAP login failed: {}", e.0)))?,
        AccountAuthMode::Oauth => {
            let token = ensure_account_access_token(state)
                .await?
                .ok_or_else(|| BackendError::validation("OAuth token is missing"))?;
            client
                .authenticate(
                    "XOAUTH2",
                    super::XOAuth2Authenticator {
                        payload: xoauth2_payload(&state.config.username, &token.access_token),
                    },
                )
                .await
                .map_err(|(e, _)| BackendError::internal(format!("IMAP OAuth login failed: {e}")))?
        }
    };

    eprintln!(
        "[imap] connected to {host}:{port} ({:.1?})",
        start.elapsed()
    );
    Ok(session)
}

pub(super) async fn imap_connect_by_account(
    account_id: &str,
) -> Result<super::ImapAnySession, BackendError> {
    let state = memory::store()
        .accounts()
        .get_account_state(account_id)
        .ok_or_else(|| BackendError::not_found("Account not found"))?;
    imap_connect(&state).await
}

pub(super) async fn imap_store_flag(
    account_id: &str,
    folder_id: &str,
    uid: u32,
    flag: &str,
    add: bool,
) -> Result<(), BackendError> {
    let mailbox_name = super::folder_ops::get_imap_folder_name(account_id, folder_id)
        .ok_or_else(|| BackendError::internal("IMAP folder name not found"))?;

    let mut session = imap_connect_by_account(account_id).await?;
    session
        .select(&mailbox_name)
        .await
        .map_err(|e| BackendError::internal(format!("IMAP SELECT failed: {e}")))?;

    let query = if add {
        format!("+FLAGS ({})", flag)
    } else {
        format!("-FLAGS ({})", flag)
    };

    session
        .uid_store(uid.to_string(), &query)
        .await
        .map_err(|e| BackendError::internal(format!("IMAP STORE failed: {e}")))?
        .try_collect::<Vec<_>>()
        .await
        .map_err(|e| BackendError::internal(format!("IMAP STORE failed: {e}")))?;

    let _ = session.logout().await;
    Ok(())
}

async fn imap_tcp_connect(host: &str, port: u16) -> Result<TcpStream, BackendError> {
    tokio::time::timeout(super::TCP_CONNECT_TIMEOUT, TcpStream::connect((host, port)))
        .await
        .map_err(|_| BackendError::internal("IMAP connection timed out"))?
        .map_err(|e| BackendError::internal(format!("IMAP connection failed: {e}")))
}

async fn imap_tls_connect(
    host: &str,
    tcp: TcpStream,
) -> Result<TlsStream<TcpStream>, BackendError> {
    let connector = native_tls::TlsConnector::new()
        .map_err(|e| BackendError::internal(format!("TLS error: {e}")))?;
    let connector = tokio_native_tls::TlsConnector::from(connector);
    connector
        .connect(host, tcp)
        .await
        .map_err(|e| BackendError::internal(format!("TLS handshake failed: {e}")))
}

async fn imap_read_greeting<T>(client: &mut async_imap::Client<T>) -> Result<(), BackendError>
where
    T: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + std::fmt::Debug + Send,
{
    let greeting = client
        .read_response()
        .await
        .map_err(|e| BackendError::internal(format!("Failed to read IMAP greeting: {e}")))?;

    if greeting.is_none() {
        return Err(BackendError::internal(
            "IMAP server closed connection before greeting",
        ));
    }

    Ok(())
}

async fn imap_read_raw_line(stream: &mut TcpStream, context: &str) -> Result<String, BackendError> {
    let mut buf = Vec::new();

    loop {
        let mut byte = [0_u8; 1];
        let read = stream
            .read(&mut byte)
            .await
            .map_err(|e| BackendError::internal(format!("{context}: {e}")))?;

        if read == 0 {
            if buf.is_empty() {
                return Err(BackendError::internal(
                    "IMAP server closed connection unexpectedly",
                ));
            }
            break;
        }

        buf.push(byte[0]);
        if buf.ends_with(b"\r\n") {
            break;
        }
    }

    String::from_utf8(buf)
        .map(|line| line.trim_end_matches(['\r', '\n']).to_string())
        .map_err(|e| BackendError::internal(format!("Invalid IMAP response bytes: {e}")))
}

async fn imap_upgrade_starttls(
    host: &str,
    mut tcp: TcpStream,
) -> Result<async_imap::Client<TlsStream<TcpStream>>, BackendError> {
    let greeting = imap_read_raw_line(&mut tcp, "Failed to read IMAP greeting").await?;
    if !greeting.starts_with("* ") {
        return Err(BackendError::internal(format!(
            "Unexpected IMAP greeting: {greeting}"
        )));
    }

    tcp.write_all(b"a001 STARTTLS\r\n").await.map_err(|e| {
        BackendError::internal(format!("Failed to send IMAP STARTTLS command: {e}"))
    })?;
    tcp.flush().await.map_err(|e| {
        BackendError::internal(format!("Failed to flush IMAP STARTTLS command: {e}"))
    })?;

    loop {
        let response =
            imap_read_raw_line(&mut tcp, "Failed to read IMAP STARTTLS response").await?;
        if response.starts_with("* ") {
            continue;
        }

        if response.starts_with("a001 OK") {
            break;
        }

        return Err(BackendError::internal(format!(
            "IMAP STARTTLS failed: {response}"
        )));
    }

    let tls_stream = imap_tls_connect(host, tcp).await?;
    Ok(async_imap::Client::new(tls_stream))
}

async fn imap_authenticate_client<T>(
    client: async_imap::Client<T>,
    draft: &AccountSetupDraft,
) -> Result<async_imap::Session<T>, BackendError>
where
    T: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + std::fmt::Debug + Send,
{
    match draft.auth_mode {
        AccountAuthMode::Password => client
            .login(draft.username.trim(), draft.password.trim())
            .await
            .map_err(|e| BackendError::validation(format!("IMAP login failed: {}", e.0))),
        AccountAuthMode::Oauth => {
            let config = AccountConfig {
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
            };
            let token = ensure_config_access_token(&config).await?;
            client
                .authenticate(
                    "XOAUTH2",
                    super::XOAuth2Authenticator {
                        payload: xoauth2_payload(&draft.username, &token.access_token),
                    },
                )
                .await
                .map_err(|(e, _)| BackendError::validation(format!("IMAP OAuth login failed: {e}")))
        }
    }
}
