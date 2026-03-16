use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use async_imap::{Authenticator, Session as ImapSession};
use async_trait::async_trait;
use base64::{Engine as _, engine::general_purpose};
use futures::TryStreamExt;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf};
use tokio::net::TcpStream;
use tokio_native_tls::TlsStream;

use crate::models::{
    AccountAuthMode, AccountConfig, AccountSetupDraft, AttachmentContent, AttachmentMeta, DraftMessage, MailAccount,
    MailFolderKind, MailMessage, MailThread, MailboxBundle, MailboxFolder, StoredAccountState, SyncStatus,
};
use crate::oauth::{ensure_account_access_token, ensure_config_access_token, xoauth2_payload};
use crate::protocol::BackendError;
use crate::provider::common::{
    validate_draft, extract_body_from_mime, extract_attachments_from_mime,
    make_preview, strip_html_tags, decode_header_value, find_mime_part_by_path,
    get_attachment_filename, base64_encode_bytes, base64_decode,
};
use crate::provider::MailProvider;
use crate::storage::memory;
use crate::storage::persisted;

const TCP_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// A stream that supports both plain TCP and TLS connections.
#[derive(Debug)]
enum ImapStream {
    Plain(TcpStream),
    Tls(TlsStream<TcpStream>),
}

impl AsyncRead for ImapStream {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            ImapStream::Plain(s) => Pin::new(s).poll_read(cx, buf),
            ImapStream::Tls(s) => Pin::new(s).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for ImapStream {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<std::io::Result<usize>> {
        match self.get_mut() {
            ImapStream::Plain(s) => Pin::new(s).poll_write(cx, buf),
            ImapStream::Tls(s) => Pin::new(s).poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            ImapStream::Plain(s) => Pin::new(s).poll_flush(cx),
            ImapStream::Tls(s) => Pin::new(s).poll_flush(cx),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            ImapStream::Plain(s) => Pin::new(s).poll_shutdown(cx),
            ImapStream::Tls(s) => Pin::new(s).poll_shutdown(cx),
        }
    }
}

type ImapAnySession = ImapSession<ImapStream>;

struct XOAuth2Authenticator {
    payload: String,
}

impl Authenticator for XOAuth2Authenticator {
    type Response = String;

    fn process(&mut self, _challenge: &[u8]) -> Self::Response {
        self.payload.clone()
    }
}

pub struct ImapSmtpProvider;

pub static IMAP_SMTP_PROVIDER: ImapSmtpProvider = ImapSmtpProvider;

#[async_trait]
impl MailProvider for ImapSmtpProvider {
    fn backend_name(&self) -> &'static str {
        "imap-smtp"
    }

    async fn list_accounts(&self) -> Result<Vec<MailAccount>, BackendError> {
        memory::list_accounts()
    }

    async fn create_account(&self, draft: AccountSetupDraft) -> Result<MailAccount, BackendError> {
        eprintln!("[imap] testing connection for new account {}...", draft.email);
        self.test_account_connection(draft.clone()).await?;
        eprintln!("[imap] connection test passed, creating account");
        memory::create_account_without_test(draft)
    }

    async fn test_account_connection(&self, draft: AccountSetupDraft) -> Result<SyncStatus, BackendError> {
        validate_draft(&draft)?;
        eprintln!("[imap] connecting to {}:{}...", draft.incoming_host, draft.incoming_port);
        let start = Instant::now();
        imap_login_test(&draft).await?;
        eprintln!("[imap] connection test ok ({:.1?})", start.elapsed());

        Ok(SyncStatus {
            account_id: "connection-test".into(),
            state: "idle".into(),
            message: format!(
                "Connected to {}:{} and {}:{}",
                draft.incoming_host, draft.incoming_port, draft.outgoing_host, draft.outgoing_port
            ),
            updated_at: memory::current_timestamp(),
        })
    }

    async fn list_folders(&self, account_id: &str) -> Result<Vec<MailboxFolder>, BackendError> {
        memory::list_folders(account_id)
    }

    async fn list_messages(&self, account_id: &str, folder_id: &str) -> Result<Vec<MailMessage>, BackendError> {
        memory::list_messages(account_id, folder_id)
    }

    async fn search_messages(&self, account_id: &str, query: &str) -> Result<Vec<MailMessage>, BackendError> {
        memory::search_messages(account_id, query)
    }

    async fn get_message(&self, account_id: &str, message_id: &str) -> Result<Option<MailMessage>, BackendError> {
        memory::get_message(account_id, message_id)
    }

    async fn save_draft(&self, draft: DraftMessage) -> Result<DraftMessage, BackendError> {
        memory::save_draft(draft)
    }

    async fn send_message(&self, draft: DraftMessage) -> Result<String, BackendError> {
        if draft.account_id.trim().is_empty() || draft.to.trim().is_empty() {
            return Err(BackendError::validation("Recipient and account are required"));
        }

        let account_state = memory::get_account_state(&draft.account_id)
            .ok_or_else(|| BackendError::not_found("Account not found"))?;

        eprintln!("[smtp] sending message to {} via {}:{}...", draft.to, account_state.config.outgoing_host, account_state.config.outgoing_port);
        let start = Instant::now();
        smtp_send(&account_state, &draft).await?;
        eprintln!("[smtp] sent ok ({:.1?})", start.elapsed());
        memory::record_sent_message(draft)
    }

    async fn toggle_star(&self, account_id: &str, message_id: &str) -> Result<Option<MailMessage>, BackendError> {
        let updated = memory::toggle_star(account_id, message_id)?;
        if let Some(ref msg) = updated {
            if let Some(uid) = msg.imap_uid {
                eprintln!("[imap] pushing star={} for uid {} in {}", msg.is_starred, uid, msg.folder_id);
                if let Err(e) = imap_store_flag(account_id, &msg.folder_id, uid, "\\Flagged", msg.is_starred).await {
                    eprintln!("[imap] push star failed: {}", e.message);
                }
            }
        }
        Ok(updated)
    }

    async fn toggle_read(&self, account_id: &str, message_id: &str) -> Result<Option<MailMessage>, BackendError> {
        let updated = memory::toggle_read(account_id, message_id)?;
        if let Some(ref msg) = updated {
            if let Some(uid) = msg.imap_uid {
                eprintln!("[imap] pushing read={} for uid {} in {}", msg.is_read, uid, msg.folder_id);
                if let Err(e) = imap_store_flag(account_id, &msg.folder_id, uid, "\\Seen", msg.is_read).await {
                    eprintln!("[imap] push read failed: {}", e.message);
                }
            }
        }
        Ok(updated)
    }

    async fn delete_message(&self, account_id: &str, message_id: &str) -> Result<(), BackendError> {
        let original = memory::get_message(account_id, message_id)?;
        memory::delete_message(account_id, message_id)?;

        if let Some(msg) = original {
            if let Some(uid) = msg.imap_uid {
                if let Ok(folders) = memory::list_folders(account_id) {
                    if let Some(trash) = folders.iter().find(|f| matches!(f.kind, MailFolderKind::Trash)) {
                        eprintln!("[imap] moving uid {} to trash", uid);
                        if let Err(e) = imap_move_message(account_id, &msg.folder_id, &trash.id, uid).await {
                            eprintln!("[imap] push delete failed: {}", e.message);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    async fn delete_account(&self, account_id: &str) -> Result<(), BackendError> {
        eprintln!("[store] deleting account {account_id}");
        memory::delete_account(account_id)
    }

    async fn archive_message(&self, account_id: &str, message_id: &str) -> Result<Option<MailMessage>, BackendError> {
        let original = memory::get_message(account_id, message_id)?;
        let updated = memory::archive_message(account_id, message_id)?;

        if let (Some(orig), Some(ref upd)) = (original, &updated) {
            if let Some(uid) = orig.imap_uid {
                eprintln!("[imap] archiving uid {} from {} to {}", uid, orig.folder_id, upd.folder_id);
                if let Err(e) = imap_move_message(account_id, &orig.folder_id, &upd.folder_id, uid).await {
                    eprintln!("[imap] push archive failed: {}", e.message);
                }
            }
        }
        Ok(updated)
    }

    async fn restore_message(&self, account_id: &str, message_id: &str) -> Result<Option<MailMessage>, BackendError> {
        let original = memory::get_message(account_id, message_id)?;
        let updated = memory::restore_message(account_id, message_id)?;

        if let (Some(orig), Some(ref upd)) = (original, &updated) {
            if let Some(uid) = orig.imap_uid {
                eprintln!("[imap] restoring uid {} from {} to {}", uid, orig.folder_id, upd.folder_id);
                if let Err(e) = imap_move_message(account_id, &orig.folder_id, &upd.folder_id, uid).await {
                    eprintln!("[imap] push restore failed: {}", e.message);
                }
            }
        }
        Ok(updated)
    }

    async fn move_message(&self, account_id: &str, message_id: &str, folder_id: &str) -> Result<Option<MailMessage>, BackendError> {
        let original = memory::get_message(account_id, message_id)?;
        let updated = memory::move_message(account_id, message_id, folder_id)?;

        if let Some(orig) = original {
            if let Some(uid) = orig.imap_uid {
                eprintln!("[imap] moving uid {} from {} to {}", uid, orig.folder_id, folder_id);
                if let Err(e) = imap_move_message(account_id, &orig.folder_id, folder_id, uid).await {
                    eprintln!("[imap] push move failed: {}", e.message);
                }
            }
        }
        Ok(updated)
    }

    async fn mark_all_read(&self, account_id: &str, folder_id: &str) -> Result<(), BackendError> {
        let unread_uids: Vec<(u32, String)> = {
            let messages = memory::list_messages(account_id, folder_id)?;
            messages
                .iter()
                .filter(|m| !m.is_read)
                .filter_map(|m| m.imap_uid.map(|uid| (uid, m.folder_id.clone())))
                .collect()
        };

        eprintln!("[store] marking {} messages read in {folder_id}", unread_uids.len());
        memory::mark_all_read(account_id, folder_id)?;

        if !unread_uids.is_empty() {
            if let Some(real_folder_id) = unread_uids.first().map(|(_, fid)| fid.clone()) {
                if let Some(mailbox_name) = get_imap_folder_name(account_id, &real_folder_id) {
                    eprintln!("[imap] pushing \\Seen for {} messages in {mailbox_name}", unread_uids.len());
                    if let Ok(mut session) = imap_connect_by_account(account_id).await {
                        if session.select(&mailbox_name).await.is_ok() {
                            for (uid, _) in &unread_uids {
                                if let Ok(stream) = session.uid_store(uid.to_string(), "+FLAGS (\\Seen)").await {
                                    let _ = stream.try_collect::<Vec<_>>().await;
                                }
                            }
                        }
                        let _ = session.logout().await;
                    }
                }
            }
        }

        Ok(())
    }

    async fn sync_account(&self, account_id: &str) -> Result<SyncStatus, BackendError> {
        let account_state = memory::get_account_state(account_id)
            .ok_or_else(|| BackendError::not_found("Account not found"))?;

        eprintln!("[imap] syncing account {} ({}:{})...", account_id, account_state.config.incoming_host, account_state.config.incoming_port);
        let start = Instant::now();
        let (folders, messages, threads) = imap_fetch_mailbox(&account_state).await?;
        eprintln!("[imap] fetched {} folders, {} messages in {:.1?}", folders.len(), messages.len(), start.elapsed());
        memory::merge_remote_mailbox(account_id, folders, messages, threads)?;

        let timestamp = memory::current_timestamp();
        memory::finish_sync(account_id, &timestamp)
    }

    async fn get_mailbox_bundle(&self, account_id: &str) -> Result<MailboxBundle, BackendError> {
        memory::get_mailbox_bundle(account_id)
    }

    async fn get_attachment_content(
        &self,
        account_id: &str,
        message_id: &str,
        attachment_id: &str,
    ) -> Result<AttachmentContent, BackendError> {
        let _ = account_id; // message_id already encodes the account
        let raw = persisted::load_raw_email(message_id)
            .map_err(|_| BackendError::not_found("Raw email not found. Try syncing the account."))?;

        let parsed = mailparse::parse_mail(&raw)
            .map_err(|e| BackendError::internal(format!("Failed to parse email: {e}")))?;

        let part = find_mime_part_by_path(&parsed, attachment_id)
            .ok_or_else(|| BackendError::not_found("Attachment part not found"))?;

        let file_name = get_attachment_filename(part).unwrap_or_else(|| "attachment".into());
        let mime_type = part.ctype.mimetype.clone();
        let raw_body = part
            .get_body_raw()
            .map_err(|e| BackendError::internal(format!("Failed to read attachment body: {e}")))?;
        let data_base64 = base64_encode_bytes(&raw_body);

        Ok(AttachmentContent {
            file_name,
            mime_type,
            data_base64,
        })
    }

    async fn get_account_config(&self, account_id: &str) -> Result<AccountSetupDraft, BackendError> {
        memory::get_account_config(account_id)
    }

    async fn update_account(&self, account_id: &str, draft: AccountSetupDraft) -> Result<MailAccount, BackendError> {
        memory::update_account(account_id, draft)
    }
}


// ---------------------------------------------------------------------------
// IMAP helpers (async)
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// IMAP helpers (async)
// ---------------------------------------------------------------------------

async fn imap_tcp_connect(host: &str, port: u16) -> Result<TcpStream, BackendError> {
    tokio::time::timeout(TCP_CONNECT_TIMEOUT, TcpStream::connect((host, port)))
        .await
        .map_err(|_| BackendError::internal("IMAP connection timed out"))?
        .map_err(|e| BackendError::internal(format!("IMAP connection failed: {e}")))
}

async fn imap_tls_connect(host: &str, tcp: TcpStream) -> Result<TlsStream<TcpStream>, BackendError> {
    let connector = native_tls::TlsConnector::new()
        .map_err(|e| BackendError::internal(format!("TLS error: {e}")))?;
    let connector = tokio_native_tls::TlsConnector::from(connector);
    connector.connect(host, tcp)
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
        return Err(BackendError::internal("IMAP server closed connection before greeting"));
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
                return Err(BackendError::internal("IMAP server closed connection unexpectedly"));
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

async fn imap_upgrade_starttls(host: &str, mut tcp: TcpStream) -> Result<async_imap::Client<TlsStream<TcpStream>>, BackendError> {
    let greeting = imap_read_raw_line(&mut tcp, "Failed to read IMAP greeting").await?;
    if !greeting.starts_with("* ") {
        return Err(BackendError::internal(format!("Unexpected IMAP greeting: {greeting}")));
    }

    tcp.write_all(b"a001 STARTTLS\r\n")
        .await
        .map_err(|e| BackendError::internal(format!("Failed to send IMAP STARTTLS command: {e}")))?;
    tcp.flush()
        .await
        .map_err(|e| BackendError::internal(format!("Failed to flush IMAP STARTTLS command: {e}")))?;

    loop {
        let response = imap_read_raw_line(&mut tcp, "Failed to read IMAP STARTTLS response").await?;
        if response.starts_with("* ") {
            continue;
        }

        if response.starts_with("a001 OK") {
            break;
        }

        return Err(BackendError::internal(format!("IMAP STARTTLS failed: {response}")));
    }

    let tls_stream = imap_tls_connect(host, tcp).await?;
    Ok(async_imap::Client::new(tls_stream))
}

async fn imap_login_test(draft: &AccountSetupDraft) -> Result<(), BackendError> {
    let host = draft.incoming_host.trim();
    let port = draft.incoming_port;

    eprintln!("[imap] tcp connecting to {host}:{port}...");
    let tcp = imap_tcp_connect(host, port).await
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
                eprintln!("[imap] implicit TLS failed, trying STARTTLS fallback: {}", tls_error.message);
                imap_upgrade_starttls(host, imap_tcp_connect(host, port).await.map_err(|e| BackendError::validation(e.message))?)
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

async fn imap_connect(state: &StoredAccountState) -> Result<ImapAnySession, BackendError> {
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
                ImapStream::Tls(client.into_inner())
            }
            Err(tls_error) => {
                eprintln!("[imap] implicit TLS failed, trying STARTTLS fallback: {}", tls_error.message);
                let client = imap_upgrade_starttls(host, imap_tcp_connect(host, port).await?).await?;
                ImapStream::Tls(client.into_inner())
            }
        }
    } else {
        let mut client = async_imap::Client::new(tcp);
        imap_read_greeting(&mut client).await?;
        ImapStream::Plain(client.into_inner())
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
                    XOAuth2Authenticator {
                        payload: xoauth2_payload(&state.config.username, &token.access_token),
                    },
                )
                .await
                .map_err(|(e, _)| BackendError::internal(format!("IMAP OAuth login failed: {e}")))?
        }
    };

    eprintln!("[imap] connected to {host}:{port} ({:.1?})", start.elapsed());
    Ok(session)
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
                    XOAuth2Authenticator {
                        payload: xoauth2_payload(&draft.username, &token.access_token),
                    },
                )
                .await
                .map_err(|(e, _)| BackendError::validation(format!("IMAP OAuth login failed: {e}")))
        }
    }
}

async fn imap_connect_by_account(account_id: &str) -> Result<ImapAnySession, BackendError> {
    let state = memory::get_account_state(account_id)
        .ok_or_else(|| BackendError::not_found("Account not found"))?;
    imap_connect(&state).await
}

fn get_imap_folder_name(account_id: &str, folder_id: &str) -> Option<String> {
    memory::list_folders(account_id)
        .ok()
        .and_then(|folders| {
            folders
                .into_iter()
                .find(|f| f.id == folder_id)
                .and_then(|f| f.imap_name)
        })
}

async fn imap_store_flag(
    account_id: &str,
    folder_id: &str,
    uid: u32,
    flag: &str,
    add: bool,
) -> Result<(), BackendError> {
    let mailbox_name = get_imap_folder_name(account_id, folder_id)
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

async fn imap_move_message(
    account_id: &str,
    src_folder_id: &str,
    dest_folder_id: &str,
    uid: u32,
) -> Result<(), BackendError> {
    let src_name = get_imap_folder_name(account_id, src_folder_id)
        .ok_or_else(|| BackendError::internal("Source IMAP folder name not found"))?;
    let dest_name = get_imap_folder_name(account_id, dest_folder_id)
        .ok_or_else(|| BackendError::internal("Destination IMAP folder name not found"))?;

    let mut session = imap_connect_by_account(account_id).await?;
    session
        .select(&src_name)
        .await
        .map_err(|e| BackendError::internal(format!("IMAP SELECT failed: {e}")))?;

    let uid_str = uid.to_string();
    session
        .uid_copy(&uid_str, &dest_name)
        .await
        .map_err(|e| BackendError::internal(format!("IMAP COPY failed: {e}")))?;

    session
        .uid_store(&uid_str, "+FLAGS (\\Deleted)")
        .await
        .map_err(|e| BackendError::internal(format!("IMAP STORE \\Deleted failed: {e}")))?
        .try_collect::<Vec<_>>()
        .await
        .map_err(|e| BackendError::internal(format!("IMAP STORE \\Deleted failed: {e}")))?;

    session
        .expunge()
        .await
        .map_err(|e| BackendError::internal(format!("IMAP EXPUNGE failed: {e}")))?
        .try_collect::<Vec<_>>()
        .await
        .map_err(|e| BackendError::internal(format!("IMAP EXPUNGE failed: {e}")))?;

    let _ = session.logout().await;
    Ok(())
}

async fn imap_fetch_mailbox(
    state: &StoredAccountState,
) -> Result<(Vec<MailboxFolder>, Vec<MailMessage>, Vec<MailThread>), BackendError> {
    let mut session = imap_connect(state).await?;
    let account_id = &state.account.id;

    // Collect existing message bodies from memory so we can skip re-downloading them.
    let existing_bodies = memory::get_existing_bodies(account_id);
    eprintln!("[imap] incremental sync: {} cached bodies available", existing_bodies.len());

    let remote_folders: Vec<_> = session
        .list(None, Some("*"))
        .await
        .map_err(|e| BackendError::internal(format!("IMAP LIST failed: {e}")))?
        .try_collect()
        .await
        .map_err(|e| BackendError::internal(format!("IMAP LIST failed: {e}")))?;

    let mut folders: Vec<MailboxFolder> = Vec::new();
    let mut all_messages: Vec<MailMessage> = Vec::new();
    let mut all_threads: Vec<MailThread> = Vec::new();

    // Always include the virtual Starred folder
    folders.push(MailboxFolder {
        id: format!("starred-{account_id}"),
        account_id: account_id.clone(),
        name: "Starred".into(),
        kind: MailFolderKind::Starred,
        unread_count: 0,
        total_count: 0,
        icon: "mdi-star-outline".into(),
        imap_name: None,
    });

    for remote_folder in &remote_folders {
        let raw_name = remote_folder.name();

        if remote_folder.attributes().iter().any(|a| matches!(a, async_imap::types::NameAttribute::NoSelect)) {
            continue;
        }

        let display_name = decode_imap_utf7(raw_name);
        let (kind, icon) = classify_folder(&display_name);
        let folder_id = format!("{}-{}", slug(raw_name), account_id);

        let (unread, total, messages, threads) =
            fetch_folder_contents(&mut session, account_id, &folder_id, raw_name, &existing_bodies).await?;

        folders.push(MailboxFolder {
            id: folder_id,
            account_id: account_id.clone(),
            name: display_name,
            kind,
            unread_count: unread,
            total_count: total,
            icon: icon.into(),
            imap_name: Some(raw_name.to_string()),
        });

        all_messages.extend(messages);
        all_threads.extend(threads);
    }

    let _ = session.logout().await;
    Ok((folders, all_messages, all_threads))
}

/// Cached body + preview + attachments for a message, keyed by IMAP UID.
type ExistingBodies = std::collections::HashMap<u32, (String, String, Vec<AttachmentMeta>, String)>;

async fn fetch_folder_contents(
    session: &mut ImapAnySession,
    account_id: &str,
    folder_id: &str,
    mailbox_name: &str,
    existing_bodies: &ExistingBodies,
) -> Result<(u32, u32, Vec<MailMessage>, Vec<MailThread>), BackendError> {
    let mailbox = session
        .select(mailbox_name)
        .await
        .map_err(|e| BackendError::internal(format!("IMAP SELECT '{mailbox_name}' failed: {e}")))?;

    let total = mailbox.exists;

    if total == 0 {
        return Ok((0, total, Vec::new(), Vec::new()));
    }

    // SEARCH UNSEEN to get the actual unread count (SELECT's UNSEEN is just
    // the sequence number of the first unseen message, not the count).
    let unread = session
        .search("UNSEEN")
        .await
        .map(|ids| ids.len() as u32)
        .unwrap_or(0);

    // Fetch the most recent 50 messages (or all if fewer)
    let start = if total > 50 { total - 49 } else { 1 };
    let range = format!("{start}:{total}");

    // --- Phase 1: lightweight fetch (envelope + flags, no body) ---
    let fetches: Vec<_> = session
        .fetch(&range, "(UID FLAGS ENVELOPE RFC822.SIZE)")
        .await
        .map_err(|e| BackendError::internal(format!("IMAP FETCH headers failed: {e}")))?
        .try_collect()
        .await
        .map_err(|e| BackendError::internal(format!("IMAP FETCH headers failed: {e}")))?;

    // Collect parsed metadata and identify which UIDs need a body download.
    struct MsgMeta {
        uid: u32,
        message_id: String,
        thread_id: String,
        is_read: bool,
        is_starred: bool,
        subject: String,
        from: String,
        from_email: String,
        to: Vec<String>,
        cc: Vec<String>,
        date: String,
    }

    let mut metas: Vec<MsgMeta> = Vec::with_capacity(fetches.len());
    let mut new_uids: Vec<u32> = Vec::new();

    for fetch in &fetches {
        let uid = fetch.uid.unwrap_or(fetch.message);
        let message_id = format!("imap-{account_id}-{folder_id}-{uid}");
        let thread_id = format!("thread-{message_id}");

        let is_read = fetch.flags().any(|f| matches!(f, async_imap::types::Flag::Seen));
        let is_starred = fetch.flags().any(|f| matches!(f, async_imap::types::Flag::Flagged));

        let (subject, from, from_email, to, cc, date) = match fetch.envelope() {
            Some(env) => {
                let (subj, frm, frm_email, t, c, d) = parse_envelope(env);
                // If envelope has no date, log a warning
                if d.is_empty() {
                    eprintln!("[imap] WARNING: no date in envelope for '{}' from {}", subj, frm);
                }
                (subj, frm, frm_email, t, c, d)
            }
            None => (
                "(No subject)".into(),
                "Unknown".into(),
                "unknown@unknown".into(),
                vec![],
                vec![],
                memory::current_timestamp(),
            ),
        };

        if !existing_bodies.contains_key(&uid) {
            new_uids.push(uid);
        }

        metas.push(MsgMeta {
            uid, message_id, thread_id, is_read, is_starred,
            subject, from, from_email, to, cc, date,
        });
    }

    // --- Phase 2: fetch full body ONLY for new messages ---
    let mut body_map: std::collections::HashMap<u32, Vec<u8>> = std::collections::HashMap::new();

    if !new_uids.is_empty() {
        eprintln!("[imap] fetching body for {} new messages in {mailbox_name}", new_uids.len());
        let uid_set = new_uids.iter().map(|u| u.to_string()).collect::<Vec<_>>().join(",");
        if let Ok(stream) = session.uid_fetch(&uid_set, "BODY.PEEK[]").await {
            if let Ok(body_fetches) = stream.try_collect::<Vec<_>>().await {
                for bf in &body_fetches {
                    if let (Some(uid), Some(raw)) = (bf.uid, bf.body()) {
                        // Save raw .eml for attachment download later
                        let msg_id = format!("imap-{account_id}-{folder_id}-{uid}");
                        let _ = persisted::save_raw_email(&msg_id, raw);
                        body_map.insert(uid, raw.to_vec());
                    }
                }
            }
        }
    } else {
        eprintln!("[imap] all {} messages cached in {mailbox_name}, skipping body fetch", metas.len());
    }

    // --- Phase 3: assemble messages ---
    let mut messages = Vec::with_capacity(metas.len());
    let mut threads = Vec::with_capacity(metas.len());

    for meta in metas {
        let (body, preview, attachments, final_date) = if meta.date.is_empty() {
            // If no date in envelope, check if we have cached data with a valid date
            if let Some((cached_body, cached_preview, cached_attachments, cached_date)) = existing_bodies.get(&meta.uid) {
                // Use cached data if we have a valid date (not current timestamp pattern)
                if !cached_date.is_empty() && !cached_date.starts_with("2026-03-16") {
                    // Cached date looks valid, use it
                    (cached_body.clone(), cached_preview.clone(), cached_attachments.clone(), cached_date.clone())
                } else if let Some(raw) = body_map.remove(&meta.uid) {
                    // Need to extract date from body
                    let parsed = extract_body_from_mime(&raw);
                    let prev = make_preview(&parsed);
                    let atts = extract_attachments_from_mime(&raw);
                    let date = extract_date_from_body(&raw, meta.uid);
                    (parsed, prev, atts, date)
                } else {
                    // No body available, use cached data with current time
                    eprintln!("[imap] WARNING: no body available for UID {} with empty date", meta.uid);
                    (cached_body.clone(), cached_preview.clone(), cached_attachments.clone(), memory::current_timestamp())
                }
            } else if let Some(raw) = body_map.remove(&meta.uid) {
                // No cache, extract from body
                let parsed = extract_body_from_mime(&raw);
                let prev = make_preview(&parsed);
                let atts = extract_attachments_from_mime(&raw);
                let date = extract_date_from_body(&raw, meta.uid);
                (parsed, prev, atts, date)
            } else {
                // No body available
                eprintln!("[imap] WARNING: no body available for UID {} with empty date", meta.uid);
                (String::new(), "(No preview)".into(), vec![], memory::current_timestamp())
            }
        } else if let Some((cached_body, cached_preview, cached_attachments, _)) = existing_bodies.get(&meta.uid) {
            // Reuse body + preview + attachments from memory, use envelope date
            (cached_body.clone(), cached_preview.clone(), cached_attachments.clone(), meta.date.clone())
        } else if let Some(raw) = body_map.remove(&meta.uid) {
            // Parse newly downloaded body and extract attachments
            let parsed = extract_body_from_mime(&raw);
            let prev = make_preview(&parsed);
            let atts = extract_attachments_from_mime(&raw);
            (parsed, prev, atts, meta.date.clone())
        } else {
            (String::new(), "(No preview)".into(), vec![], meta.date.clone())
        };

        messages.push(MailMessage {
            id: meta.message_id.clone(),
            account_id: account_id.into(),
            folder_id: folder_id.into(),
            thread_id: meta.thread_id.clone(),
            subject: meta.subject.clone(),
            preview,
            body,
            from: meta.from,
            from_email: meta.from_email,
            to: meta.to,
            cc: meta.cc,
            sent_at: final_date.clone(),
            received_at: final_date,
            is_read: meta.is_read,
            is_starred: meta.is_starred,
            has_attachments: !attachments.is_empty(),
            attachments,
            labels: vec![],
            imap_uid: Some(meta.uid),
        });

        threads.push(MailThread {
            id: meta.thread_id,
            account_id: account_id.into(),
            subject: meta.subject,
            message_ids: vec![meta.message_id],
            last_message_at: memory::current_timestamp(),
            unread_count: if meta.is_read { 0 } else { 1 },
        });
    }

    Ok((unread, total, messages, threads))
}

fn classify_folder(name: &str) -> (MailFolderKind, &'static str) {
    let lower = name.to_lowercase();
    if lower == "inbox" || lower == "收件箱" {
        (MailFolderKind::Inbox, "mdi-inbox-arrow-down")
    } else if lower.contains("sent") || lower.contains("已发送") {
        (MailFolderKind::Sent, "mdi-send-outline")
    } else if lower.contains("draft") || lower.contains("草稿") {
        (MailFolderKind::Drafts, "mdi-file-document-edit-outline")
    } else if lower.contains("trash") || lower.contains("deleted") || lower.contains("已删除") {
        (MailFolderKind::Trash, "mdi-delete-outline")
    } else if lower.contains("archive") || lower.contains("all mail") || lower == "[gmail]/all mail" || lower.contains("归档") {
        (MailFolderKind::Archive, "mdi-archive-outline")
    } else if lower.contains("spam") || lower.contains("junk") || lower.contains("垃圾") {
        (MailFolderKind::Trash, "mdi-alert-circle-outline")
    } else {
        (MailFolderKind::Custom, "mdi-folder-outline")
    }
}

fn slug(name: &str) -> String {
    name.to_lowercase()
        .replace(|c: char| !c.is_alphanumeric(), "-")
        .trim_matches('-')
        .to_string()
}

fn decode_imap_utf7(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] != b'&' {
            result.push(bytes[i] as char);
            i += 1;
            continue;
        }

        i += 1;
        if i >= bytes.len() {
            result.push('&');
            break;
        }

        if bytes[i] == b'-' {
            result.push('&');
            i += 1;
            continue;
        }

        let start = i;
        while i < bytes.len() && bytes[i] != b'-' {
            i += 1;
        }

        if i >= bytes.len() {
            result.push('&');
            result.push_str(&String::from_utf8_lossy(&bytes[start..]));
            break;
        }

        let encoded = &bytes[start..i];
        i += 1;

        let decoded_utf16 = decode_modified_base64(encoded);
        if let Ok(s) = String::from_utf16(&decoded_utf16) {
            result.push_str(&s);
        } else {
            result.push('&');
            result.push_str(&String::from_utf8_lossy(encoded));
            result.push('-');
        }
    }

    result
}

fn decode_modified_base64(input: &[u8]) -> Vec<u16> {
    let mut input_str = String::from_utf8_lossy(input).to_string();
    input_str = input_str.replace(',', "/");

    // Pad to multiple of 4
    while input_str.len() % 4 != 0 {
        input_str.push('=');
    }

    // Standard base64 decode
    let decoded_bytes = match general_purpose::STANDARD.decode(&input_str) {
        Ok(bytes) => bytes,
        Err(_) => return Vec::new(),
    };

    // Convert bytes to UTF-16 big-endian code units
    let mut out = Vec::new();
    for chunk in decoded_bytes.chunks(2) {
        if chunk.len() == 2 {
            out.push(u16::from_be_bytes([chunk[0], chunk[1]]));
        }
    }

    out
}

fn extract_date_from_body(raw: &[u8], _uid: u32) -> String {
    match mailparse::parse_mail(raw) {
        Ok(mail) => {
            // First try Date header
            let date_header = mail.headers.iter()
                .find(|h| h.get_key().eq_ignore_ascii_case("Date"))
                .map(|h| h.get_value());

            if let Some(date_str) = date_header {
                match mailparse::dateparse(&date_str) {
                    Ok(ts) => {
                        let dt = chrono::DateTime::from_timestamp(ts, 0)
                            .unwrap_or_else(|| chrono::Utc::now());
                        return dt.to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
                    }
                    Err(_) => {}
                }
            }

            // No Date header, try to extract from first Received header
            let received_header = mail.headers.iter()
                .find(|h| h.get_key().eq_ignore_ascii_case("Received"))
                .map(|h| h.get_value());

            if let Some(received_str) = received_header {
                // Received header format: "... ; date"
                if let Some(date_part) = received_str.split(';').last() {
                    let date_part = date_part.trim();
                    match mailparse::dateparse(date_part) {
                        Ok(ts) => {
                            let dt = chrono::DateTime::from_timestamp(ts, 0)
                                .unwrap_or_else(|| chrono::Utc::now());
                            return dt.to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
                        }
                        Err(_) => {}
                    }
                }
            }

            memory::current_timestamp()
        }
        Err(_) => memory::current_timestamp()
    }
}

fn parse_envelope(env: &imap_proto::types::Envelope) -> (String, String, String, Vec<String>, Vec<String>, String) {
    let subject = env
        .subject
        .as_ref()
        .map(|s| decode_header_value(s))
        .unwrap_or_else(|| "(No subject)".into());

    let (from, from_email) = env
        .from
        .as_ref()
        .and_then(|addrs| addrs.first())
        .map(|addr: &imap_proto::types::Address| {
            let name = addr
                .name
                .as_ref()
                .map(|n| decode_header_value(n))
                .unwrap_or_default();
            let mailbox = addr.mailbox.as_ref().map(|m| String::from_utf8_lossy(m).to_string()).unwrap_or_default();
            let host = addr.host.as_ref().map(|h| String::from_utf8_lossy(h).to_string()).unwrap_or_default();
            let email = format!("{mailbox}@{host}");
            let display = if name.is_empty() { email.clone() } else { name };
            (display, email)
        })
        .unwrap_or_else(|| ("Unknown".into(), "unknown@unknown".into()));

    let extract_addresses = |addrs: &[imap_proto::types::Address]| -> Vec<String> {
        addrs
            .iter()
            .map(|addr| {
                let mailbox = addr.mailbox.as_ref().map(|m| String::from_utf8_lossy(m).to_string()).unwrap_or_default();
                let host = addr.host.as_ref().map(|h| String::from_utf8_lossy(h).to_string()).unwrap_or_default();
                format!("{mailbox}@{host}")
            })
            .collect()
    };

    let to = env.to.as_ref().map(|a| extract_addresses(a)).unwrap_or_default();
    let cc = env.cc.as_ref().map(|a| extract_addresses(a)).unwrap_or_default();

    let date = env
        .date
        .as_ref()
        .map(|d| {
            let raw = String::from_utf8_lossy(d).to_string();
            // Try to parse RFC 2822 date and convert to ISO 8601
            mailparse::dateparse(&raw)
                .map(|ts| {
                    let secs = ts;
                    let dt = chrono::DateTime::from_timestamp(secs, 0)
                        .unwrap_or_else(|| chrono::Utc::now());
                    dt.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
                })
                .unwrap_or_else(|e| {
                    eprintln!("[imap] WARNING: failed to parse date '{}': {:?}", raw, e);
                    String::new()  // Return empty string instead of current time
                })
        })
        .unwrap_or_else(|| String::new());  // Return empty string if no date field

    (subject, from, from_email, to, cc, date)
}

// SMTP helper (async)
// ---------------------------------------------------------------------------

pub async fn smtp_send(state: &StoredAccountState, draft: &DraftMessage) -> Result<(), BackendError> {
    use lettre::message::header::ContentType;
    use lettre::message::{Attachment, Mailbox, MultiPart, SinglePart};
    use lettre::transport::smtp::authentication::{Credentials, Mechanism};
    use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};

    let from: Mailbox = format!("{} <{}>", state.account.name, state.account.email)
        .parse()
        .map_err(|e| BackendError::validation(format!("Invalid sender address: {e}")))?;

    let mut builder = Message::builder().from(from);

    for recipient in draft.to.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        let to: Mailbox = recipient
            .parse()
            .map_err(|e| BackendError::validation(format!("Invalid recipient '{recipient}': {e}")))?;
        builder = builder.to(to);
    }

    for recipient in draft.cc.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        let cc: Mailbox = recipient
            .parse()
            .map_err(|e| BackendError::validation(format!("Invalid CC '{recipient}': {e}")))?;
        builder = builder.cc(cc);
    }

    for recipient in draft.bcc.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        let bcc: Mailbox = recipient
            .parse()
            .map_err(|e| BackendError::validation(format!("Invalid BCC '{recipient}': {e}")))?;
        builder = builder.bcc(bcc);
    }

    let email = if draft.attachments.is_empty() {
        let plain_text = strip_html_tags(&draft.body);
        let alternative = MultiPart::alternative()
            .singlepart(SinglePart::plain(plain_text))
            .singlepart(SinglePart::html(draft.body.clone()));
        builder
            .subject(&draft.subject)
            .multipart(alternative)
            .map_err(|e| BackendError::internal(format!("Failed to build email: {e}")))?
    } else {
        let plain_text = strip_html_tags(&draft.body);
        let alternative = MultiPart::alternative()
            .singlepart(SinglePart::plain(plain_text))
            .singlepart(SinglePart::html(draft.body.clone()));
        let mut multipart = MultiPart::mixed().multipart(alternative);

        for att in &draft.attachments {
            let decoded = base64_decode(&att.data_base64).unwrap_or_default();
            let content_type = ContentType::parse(&att.mime_type)
                .unwrap_or(ContentType::TEXT_PLAIN);
            let attachment = Attachment::new(att.file_name.clone()).body(decoded, content_type);
            multipart = multipart.singlepart(attachment);
        }

        builder
            .subject(&draft.subject)
            .multipart(multipart)
            .map_err(|e| BackendError::internal(format!("Failed to build email: {e}")))?
    };

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
        AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(host)
            .port(port)
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

    Ok(())
}
