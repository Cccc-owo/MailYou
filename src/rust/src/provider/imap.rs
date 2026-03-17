use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use async_imap::{Authenticator, Session as ImapSession};
use async_trait::async_trait;
use base64::{engine::general_purpose, Engine as _};
use futures::TryStreamExt;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf};
use tokio::net::TcpStream;
use tokio_native_tls::TlsStream;

use crate::models::{
    AccountAuthMode, AccountConfig, AccountSetupDraft, AttachmentContent, AttachmentMeta,
    DraftMessage, MailAccount, MailFolderKind, MailIdentity, MailLabel, MailMessage, MailThread,
    MailboxBundle, MailboxFolder, StoredAccountState, SyncStatus,
};
use crate::oauth::{ensure_account_access_token, ensure_config_access_token, xoauth2_payload};
use crate::protocol::BackendError;
use crate::provider::common::{
    base64_decode, base64_encode_bytes, decode_header_value, extract_attachments_from_mime,
    extract_body_from_mime, find_mime_part_by_path, get_attachment_filename, make_preview,
    strip_html_tags, validate_draft,
};
use crate::provider::MailProvider;
use crate::storage::memory;
use crate::storage::persisted;

const TCP_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const DRAFT_HEADER_IDENTITY: &str = "X-MailYou-Draft-Identity";
const DRAFT_HEADER_REPLY: &str = "X-MailYou-Draft-In-Reply-To";
const DRAFT_HEADER_FORWARD: &str = "X-MailYou-Draft-Forward-From";
const DRAFT_HEADER_BCC: &str = "X-MailYou-Draft-Bcc";

/// A stream that supports both plain TCP and TLS connections.
#[derive(Debug)]
enum ImapStream {
    Plain(TcpStream),
    Tls(TlsStream<TcpStream>),
}

impl AsyncRead for ImapStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            ImapStream::Plain(s) => Pin::new(s).poll_read(cx, buf),
            ImapStream::Tls(s) => Pin::new(s).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for ImapStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
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
type ExistingBodies = std::collections::HashMap<u32, (String, String, Vec<AttachmentMeta>, String)>;

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
    labels: Vec<String>,
}

struct FolderSyncResult {
    unread: u32,
    total: u32,
    uid_validity: Option<u32>,
    uid_next: Option<u32>,
    highest_modseq: Option<u64>,
    vanished_uids: Vec<u32>,
    messages: Vec<MailMessage>,
    threads: Vec<MailThread>,
    fetched_all_messages: bool,
}

pub(crate) enum IdleMailboxChange {
    Timeout,
    Changed,
    Vanished(Vec<u32>),
}

struct MailboxSelectResult {
    mailbox: async_imap::types::Mailbox,
    vanished_uids: Vec<u32>,
}

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
        eprintln!(
            "[imap] testing connection for new account {}...",
            draft.email
        );
        self.test_account_connection(draft.clone()).await?;
        eprintln!("[imap] connection test passed, creating account");
        memory::create_account_without_test(draft)
    }

    async fn test_account_connection(
        &self,
        draft: AccountSetupDraft,
    ) -> Result<SyncStatus, BackendError> {
        validate_draft(&draft)?;
        eprintln!(
            "[imap] connecting to {}:{}...",
            draft.incoming_host, draft.incoming_port
        );
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

    async fn create_folder(
        &self,
        account_id: &str,
        name: &str,
    ) -> Result<Vec<MailboxFolder>, BackendError> {
        let mailbox_name = encode_imap_utf7(name.trim());
        let mut session = imap_connect_by_account(account_id).await?;
        session
            .create(&mailbox_name)
            .await
            .map_err(|e| BackendError::internal(format!("IMAP CREATE failed: {e}")))?;
        let _ = session.logout().await;

        self.sync_account(account_id).await?;
        memory::list_folders(account_id)
    }

    async fn rename_folder(
        &self,
        account_id: &str,
        folder_id: &str,
        name: &str,
    ) -> Result<Vec<MailboxFolder>, BackendError> {
        let folder = memory::get_folder(account_id, folder_id)?;
        if !matches!(folder.kind, MailFolderKind::Custom) {
            return Err(BackendError::validation(
                "Only custom folders can be renamed",
            ));
        }

        let current_name = folder
            .imap_name
            .clone()
            .unwrap_or_else(|| encode_imap_utf7(&folder.name));
        let next_name = encode_imap_utf7(name.trim());
        let mut session = imap_connect_by_account(account_id).await?;
        session
            .rename(&current_name, &next_name)
            .await
            .map_err(|e| BackendError::internal(format!("IMAP RENAME failed: {e}")))?;
        let _ = session.logout().await;

        self.sync_account(account_id).await?;
        memory::list_folders(account_id)
    }

    async fn delete_folder(
        &self,
        account_id: &str,
        folder_id: &str,
    ) -> Result<Vec<MailboxFolder>, BackendError> {
        let folder = memory::get_folder(account_id, folder_id)?;
        if !matches!(folder.kind, MailFolderKind::Custom) {
            return Err(BackendError::validation(
                "Only custom folders can be deleted",
            ));
        }
        if folder.total_count > 0 {
            return Err(BackendError::validation(
                "Only empty folders can be deleted",
            ));
        }

        let mailbox_name = folder
            .imap_name
            .clone()
            .unwrap_or_else(|| encode_imap_utf7(&folder.name));
        let mut session = imap_connect_by_account(account_id).await?;
        session
            .delete(&mailbox_name)
            .await
            .map_err(|e| BackendError::internal(format!("IMAP DELETE failed: {e}")))?;
        let _ = session.logout().await;

        self.sync_account(account_id).await?;
        memory::list_folders(account_id)
    }

    async fn list_messages(
        &self,
        account_id: &str,
        folder_id: &str,
    ) -> Result<Vec<MailMessage>, BackendError> {
        memory::list_messages(account_id, folder_id)
    }

    async fn get_draft(
        &self,
        account_id: &str,
        draft_id: &str,
    ) -> Result<Option<DraftMessage>, BackendError> {
        if let Some(draft) = memory::get_draft(account_id, draft_id)? {
            return Ok(Some(draft));
        }

        let Some(message) = memory::get_message(account_id, draft_id)? else {
            return Ok(None);
        };
        let folder = memory::get_folder(account_id, &message.folder_id)?;
        if !matches!(folder.kind, MailFolderKind::Drafts) {
            return Ok(None);
        }

        Ok(Some(materialize_remote_draft(account_id, &message).await?))
    }

    async fn search_messages(
        &self,
        account_id: &str,
        query: &str,
    ) -> Result<Vec<MailMessage>, BackendError> {
        memory::search_messages(account_id, query)
    }

    async fn list_labels(&self, account_id: &str) -> Result<Vec<MailLabel>, BackendError> {
        imap_list_labels(account_id).await
    }

    async fn get_message(
        &self,
        account_id: &str,
        message_id: &str,
    ) -> Result<Option<MailMessage>, BackendError> {
        memory::get_message(account_id, message_id)
    }

    async fn add_label(
        &self,
        account_id: &str,
        message_id: &str,
        label: &str,
    ) -> Result<Option<MailMessage>, BackendError> {
        let Some(message) = memory::get_message(account_id, message_id)? else {
            return Ok(None);
        };
        let uid = message.imap_uid.ok_or_else(|| {
            BackendError::validation("This message does not have a remote IMAP UID")
        })?;
        imap_store_server_label(account_id, &message.folder_id, uid, label, true).await?;
        memory::add_label(account_id, message_id, label)
    }

    async fn remove_label(
        &self,
        account_id: &str,
        message_id: &str,
        label: &str,
    ) -> Result<Option<MailMessage>, BackendError> {
        let Some(message) = memory::get_message(account_id, message_id)? else {
            return Ok(None);
        };
        let uid = message.imap_uid.ok_or_else(|| {
            BackendError::validation("This message does not have a remote IMAP UID")
        })?;
        imap_store_server_label(account_id, &message.folder_id, uid, label, false).await?;
        memory::remove_label(account_id, message_id, label)
    }

    async fn rename_label(
        &self,
        account_id: &str,
        label: &str,
        new_label: &str,
    ) -> Result<Vec<MailLabel>, BackendError> {
        if imap_account_uses_gmail_labels(account_id).await? {
            imap_rename_gmail_label(account_id, label, new_label).await?;
            self.sync_account(account_id).await?;
            return imap_list_labels(account_id).await;
        }

        imap_rename_keyword_label(account_id, label, new_label).await?;
        self.sync_account(account_id).await?;
        imap_list_labels(account_id).await
    }

    async fn delete_label(
        &self,
        account_id: &str,
        label: &str,
    ) -> Result<Vec<MailLabel>, BackendError> {
        if imap_account_uses_gmail_labels(account_id).await? {
            imap_delete_gmail_label(account_id, label).await?;
            self.sync_account(account_id).await?;
            return imap_list_labels(account_id).await;
        }

        imap_delete_keyword_label(account_id, label).await?;
        self.sync_account(account_id).await?;
        imap_list_labels(account_id).await
    }

    async fn save_draft(&self, draft: DraftMessage) -> Result<DraftMessage, BackendError> {
        if draft.account_id.trim().is_empty() {
            return Err(BackendError::validation("Account is required"));
        }

        let account_state = memory::get_account_state(&draft.account_id)
            .ok_or_else(|| BackendError::not_found("Account not found"))?;
        let mailbox_name = get_drafts_mailbox_name(&draft.account_id)?;
        let raw_email = build_rfc822_message(&account_state, &draft)?.formatted();

        if let Some(existing_message) = memory::get_message(&draft.account_id, &draft.id)? {
            let existing_folder = memory::get_folder(&draft.account_id, &existing_message.folder_id)?;
            if matches!(existing_folder.kind, MailFolderKind::Drafts) {
                if let Some(uid) = existing_message.imap_uid {
                    imap_delete_message_by_uid(&draft.account_id, &existing_folder.id, uid).await?;
                }
            }
        }

        let mut session = imap_connect_by_account(&draft.account_id).await?;
        session
            .append(&mailbox_name, Some("(\\Draft)"), None, &raw_email)
            .await
            .map_err(|error| BackendError::internal(format!("IMAP APPEND draft failed: {error}")))?;
        let _ = session.logout().await;

        let _ = memory::remove_draft(&draft.account_id, &draft.id);
        self.sync_account(&draft.account_id).await?;

        if let Some(remote) = find_matching_remote_draft(&draft.account_id, &draft)? {
            return materialize_remote_draft(&draft.account_id, &remote).await;
        }

        Ok(draft)
    }

    async fn send_message(&self, draft: DraftMessage) -> Result<String, BackendError> {
        if draft.account_id.trim().is_empty() || draft.to.trim().is_empty() {
            return Err(BackendError::validation(
                "Recipient and account are required",
            ));
        }

        let account_state = memory::get_account_state(&draft.account_id)
            .ok_or_else(|| BackendError::not_found("Account not found"))?;

        eprintln!(
            "[smtp] sending message to {} via {}:{}...",
            draft.to, account_state.config.outgoing_host, account_state.config.outgoing_port
        );
        let start = Instant::now();
        let raw_email = smtp_send(&account_state, &draft).await?;
        eprintln!("[smtp] sent ok ({:.1?})", start.elapsed());
        let (message_id, queued_at) = memory::record_sent_message(draft)?;
        let _ = persisted::save_raw_email(&message_id, &raw_email);
        Ok(queued_at)
    }

    async fn toggle_star(
        &self,
        account_id: &str,
        message_id: &str,
    ) -> Result<Option<MailMessage>, BackendError> {
        let updated = memory::toggle_star(account_id, message_id)?;
        if let Some(ref msg) = updated {
            if let Some(uid) = msg.imap_uid {
                eprintln!(
                    "[imap] pushing star={} for uid {} in {}",
                    msg.is_starred, uid, msg.folder_id
                );
                if let Err(e) =
                    imap_store_flag(account_id, &msg.folder_id, uid, "\\Flagged", msg.is_starred)
                        .await
                {
                    eprintln!("[imap] push star failed: {}", e.message);
                }
            }
        }
        Ok(updated)
    }

    async fn toggle_read(
        &self,
        account_id: &str,
        message_id: &str,
    ) -> Result<Option<MailMessage>, BackendError> {
        let updated = memory::toggle_read(account_id, message_id)?;
        if let Some(ref msg) = updated {
            if let Some(uid) = msg.imap_uid {
                eprintln!(
                    "[imap] pushing read={} for uid {} in {}",
                    msg.is_read, uid, msg.folder_id
                );
                if let Err(e) =
                    imap_store_flag(account_id, &msg.folder_id, uid, "\\Seen", msg.is_read).await
                {
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
                    if let Some(trash) = folders
                        .iter()
                        .find(|f| matches!(f.kind, MailFolderKind::Trash))
                    {
                        eprintln!("[imap] moving uid {} to trash", uid);
                        if let Err(e) =
                            imap_move_message(account_id, &msg.folder_id, &trash.id, uid).await
                        {
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

    async fn archive_message(
        &self,
        account_id: &str,
        message_id: &str,
    ) -> Result<Option<MailMessage>, BackendError> {
        let original = memory::get_message(account_id, message_id)?;
        let updated = memory::archive_message(account_id, message_id)?;

        if let (Some(orig), Some(ref upd)) = (original, &updated) {
            if let Some(uid) = orig.imap_uid {
                eprintln!(
                    "[imap] archiving uid {} from {} to {}",
                    uid, orig.folder_id, upd.folder_id
                );
                if let Err(e) =
                    imap_move_message(account_id, &orig.folder_id, &upd.folder_id, uid).await
                {
                    eprintln!("[imap] push archive failed: {}", e.message);
                }
            }
        }
        Ok(updated)
    }

    async fn restore_message(
        &self,
        account_id: &str,
        message_id: &str,
    ) -> Result<Option<MailMessage>, BackendError> {
        let original = memory::get_message(account_id, message_id)?;
        let updated = memory::restore_message(account_id, message_id)?;

        if let (Some(orig), Some(ref upd)) = (original, &updated) {
            if let Some(uid) = orig.imap_uid {
                eprintln!(
                    "[imap] restoring uid {} from {} to {}",
                    uid, orig.folder_id, upd.folder_id
                );
                if let Err(e) =
                    imap_move_message(account_id, &orig.folder_id, &upd.folder_id, uid).await
                {
                    eprintln!("[imap] push restore failed: {}", e.message);
                }
            }
        }
        Ok(updated)
    }

    async fn move_message(
        &self,
        account_id: &str,
        message_id: &str,
        folder_id: &str,
    ) -> Result<Option<MailMessage>, BackendError> {
        let original = memory::get_message(account_id, message_id)?;
        let updated = memory::move_message(account_id, message_id, folder_id)?;

        if let Some(orig) = original {
            if let Some(uid) = orig.imap_uid {
                eprintln!(
                    "[imap] moving uid {} from {} to {}",
                    uid, orig.folder_id, folder_id
                );
                if let Err(e) = imap_move_message(account_id, &orig.folder_id, folder_id, uid).await
                {
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

        eprintln!(
            "[store] marking {} messages read in {folder_id}",
            unread_uids.len()
        );
        memory::mark_all_read(account_id, folder_id)?;

        if !unread_uids.is_empty() {
            if let Some(real_folder_id) = unread_uids.first().map(|(_, fid)| fid.clone()) {
                if let Some(mailbox_name) = get_imap_folder_name(account_id, &real_folder_id) {
                    eprintln!(
                        "[imap] pushing \\Seen for {} messages in {mailbox_name}",
                        unread_uids.len()
                    );
                    if let Ok(mut session) = imap_connect_by_account(account_id).await {
                        if session.select(&mailbox_name).await.is_ok() {
                            for (uid, _) in &unread_uids {
                                if let Ok(stream) =
                                    session.uid_store(uid.to_string(), "+FLAGS (\\Seen)").await
                                {
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

        eprintln!(
            "[imap] syncing account {} ({}:{})...",
            account_id, account_state.config.incoming_host, account_state.config.incoming_port
        );
        let start = Instant::now();
        let (folders, messages, threads) = imap_fetch_mailbox(&account_state).await?;
        eprintln!(
            "[imap] fetched {} folders, {} messages in {:.1?}",
            folders.len(),
            messages.len(),
            start.elapsed()
        );
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
        if let Some(content) =
            memory::get_local_attachment_content(account_id, message_id, attachment_id)?
        {
            return Ok(content);
        }
        let raw = persisted::load_raw_email(message_id).map_err(|_| {
            BackendError::not_found("Raw email not found. Try syncing the account.")
        })?;

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

    async fn get_account_config(
        &self,
        account_id: &str,
    ) -> Result<AccountSetupDraft, BackendError> {
        memory::get_account_config(account_id)
    }

    async fn update_account(
        &self,
        account_id: &str,
        draft: AccountSetupDraft,
    ) -> Result<MailAccount, BackendError> {
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

async fn imap_login_test(draft: &AccountSetupDraft) -> Result<(), BackendError> {
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
                eprintln!(
                    "[imap] implicit TLS failed, trying STARTTLS fallback: {}",
                    tls_error.message
                );
                let client =
                    imap_upgrade_starttls(host, imap_tcp_connect(host, port).await?).await?;
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

    eprintln!(
        "[imap] connected to {host}:{port} ({:.1?})",
        start.elapsed()
    );
    Ok(session)
}

pub(crate) async fn wait_for_mailbox_change(
    state: &StoredAccountState,
    mailbox_name: &str,
    idle_timeout: Duration,
) -> Result<IdleMailboxChange, BackendError> {
    let mut session = imap_connect(state).await?;
    let account_id = &state.account.id;
    let folder = memory::list_folders(account_id).ok().and_then(|folders| {
        folders
            .into_iter()
            .find(|folder| folder.imap_name.as_deref() == Some(mailbox_name))
    });
    let folder_id = folder
        .as_ref()
        .map(|folder| folder.id.as_str())
        .unwrap_or(mailbox_name);
    let _ = select_mailbox_for_incremental_sync(
        &mut session,
        account_id,
        folder_id,
        mailbox_name,
        folder.as_ref(),
    )
    .await?;

    let mut idle = session.idle();
    idle.init()
        .await
        .map_err(|error| BackendError::internal(format!("IMAP IDLE init failed: {error}")))?;

    let (wait, _interrupt) = idle.wait_with_timeout(idle_timeout);
    let response = wait
        .await
        .map_err(|error| BackendError::internal(format!("IMAP IDLE wait failed: {error}")))?;

    let mut session = idle
        .done()
        .await
        .map_err(|error| BackendError::internal(format!("IMAP IDLE done failed: {error}")))?;
    let _ = session.logout().await;

    match response {
        async_imap::extensions::idle::IdleResponse::Timeout => Ok(IdleMailboxChange::Timeout),
        async_imap::extensions::idle::IdleResponse::ManualInterrupt => {
            Ok(IdleMailboxChange::Changed)
        }
        async_imap::extensions::idle::IdleResponse::NewData(data) => match data.parsed() {
            imap_proto::Response::Vanished { uids, .. } => {
                let flattened = uids
                    .iter()
                    .flat_map(|range| range.clone())
                    .collect::<Vec<_>>();
                Ok(IdleMailboxChange::Vanished(flattened))
            }
            _ => Ok(IdleMailboxChange::Changed),
        },
    }
}

pub(crate) async fn sync_mailbox_incremental(
    account_id: &str,
    mailbox_name: &str,
) -> Result<SyncStatus, BackendError> {
    let account_state = memory::get_account_state(account_id)
        .ok_or_else(|| BackendError::not_found("Account not found"))?;
    let folder = resolve_or_create_folder(account_id, mailbox_name);
    let existing_bodies = memory::get_existing_bodies_for_folder(account_id, &folder.id);
    let known_max_uid = memory::get_max_imap_uid_for_folder(account_id, &folder.id);

    let mut session = imap_connect(&account_state).await?;
    let sync_result = fetch_folder_contents_incremental(
        &mut session,
        account_id,
        &folder.id,
        mailbox_name,
        &existing_bodies,
        known_max_uid,
        50,
    )
    .await?;
    let _ = session.logout().await;

    if !sync_result.vanished_uids.is_empty() {
        memory::remove_messages_by_imap_uids(account_id, &folder.id, &sync_result.vanished_uids)?;
    }

    let synced_folder = MailboxFolder {
        unread_count: sync_result.unread,
        total_count: sync_result.total,
        imap_uid_validity: sync_result.uid_validity,
        imap_uid_next: sync_result.uid_next,
        imap_highest_modseq: sync_result.highest_modseq,
        ..folder
    };

    memory::merge_remote_folder(
        account_id,
        synced_folder,
        sync_result.messages,
        sync_result.threads,
        sync_result.fetched_all_messages,
    )?;

    let timestamp = memory::current_timestamp();
    memory::finish_sync(account_id, &timestamp)
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
    memory::list_folders(account_id).ok().and_then(|folders| {
        folders
            .into_iter()
            .find(|f| f.id == folder_id)
            .and_then(|f| f.imap_name)
    })
}

fn get_drafts_mailbox_name(account_id: &str) -> Result<String, BackendError> {
    let folder = memory::list_folders(account_id)?
        .into_iter()
        .find(|folder| matches!(folder.kind, MailFolderKind::Drafts))
        .ok_or_else(|| BackendError::not_found("Drafts folder not found"))?;
    Ok(folder
        .imap_name
        .unwrap_or_else(|| encode_imap_utf7(&folder.name)))
}

fn normalize_recipient_list(value: &[String]) -> Vec<String> {
    value
        .iter()
        .map(|recipient| recipient.trim().to_lowercase())
        .filter(|recipient| !recipient.is_empty())
        .collect()
}

fn split_recipients_for_match(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(|recipient| recipient.trim().to_string())
        .filter(|recipient| !recipient.is_empty())
        .collect()
}

fn find_matching_remote_draft(
    account_id: &str,
    draft: &DraftMessage,
) -> Result<Option<MailMessage>, BackendError> {
    let drafts_folder = memory::list_folders(account_id)?
        .into_iter()
        .find(|folder| matches!(folder.kind, MailFolderKind::Drafts))
        .ok_or_else(|| BackendError::not_found("Drafts folder not found"))?;
    let mut messages = memory::list_messages(account_id, &drafts_folder.id)?;
    let target_to = normalize_recipient_list(&split_recipients_for_match(&draft.to));
    let target_cc = normalize_recipient_list(&split_recipients_for_match(&draft.cc));
    messages.sort_by(|left, right| right.received_at.cmp(&left.received_at));
    Ok(messages.into_iter().find(|message| {
        message.imap_uid.is_some()
            && message.subject == draft.subject
            && message.body == draft.body
            && normalize_recipient_list(&message.to) == target_to
            && normalize_recipient_list(&message.cc) == target_cc
    }))
}

async fn materialize_remote_draft(
    account_id: &str,
    message: &MailMessage,
) -> Result<DraftMessage, BackendError> {
    let account = memory::get_account_state(account_id)
        .ok_or_else(|| BackendError::not_found("Account not found"))?
        .account;
    let raw = persisted::load_raw_email(&message.id).ok();
    let parsed = raw
        .as_ref()
        .and_then(|raw_email| mailparse::parse_mail(raw_email).ok());
    let selected_identity_id = parsed
        .as_ref()
        .and_then(|mail| parse_text_header(mail, DRAFT_HEADER_IDENTITY))
        .filter(|value| !value.is_empty())
        .or_else(|| {
            account
                .identities
                .iter()
                .find(|identity| identity.email.eq_ignore_ascii_case(&message.from_email))
                .map(|identity| identity.id.clone())
        });
    let mut attachments = Vec::with_capacity(message.attachments.len());
    for attachment in &message.attachments {
        let content = get_attachment_content_from_storage(account_id, &message.id, &attachment.id)?;
        attachments.push(crate::models::DraftAttachment {
            file_name: content.file_name,
            mime_type: content.mime_type,
            data_base64: content.data_base64,
        });
    }
    let bcc = parsed
        .as_ref()
        .and_then(|mail| {
            parse_text_header(mail, DRAFT_HEADER_BCC)
                .or_else(|| parse_address_header(mail, "bcc"))
        })
        .unwrap_or_default();
    let in_reply_to_message_id = parsed
        .as_ref()
        .and_then(|mail| {
            parse_text_header(mail, DRAFT_HEADER_REPLY)
                .or_else(|| parse_text_header(mail, "in-reply-to"))
        })
        .filter(|value| !value.is_empty());
    let forward_from_message_id = parsed
        .as_ref()
        .and_then(|mail| parse_text_header(mail, DRAFT_HEADER_FORWARD))
        .filter(|value| !value.is_empty());

    Ok(DraftMessage {
        id: message.id.clone(),
        account_id: account_id.to_string(),
        selected_identity_id,
        to: message.to.join(", "),
        cc: message.cc.join(", "),
        bcc,
        subject: message.subject.clone(),
        body: message.body.clone(),
        in_reply_to_message_id,
        forward_from_message_id,
        attachments,
    })
}

fn parse_text_header(mail: &mailparse::ParsedMail<'_>, header_name: &str) -> Option<String> {
    mail.headers
        .iter()
        .find(|header| header.get_key().eq_ignore_ascii_case(header_name))
        .map(|header| decode_header_value(header.get_value_raw()))
        .map(|value| value.trim().to_string())
}

fn parse_address_header(mail: &mailparse::ParsedMail<'_>, header_name: &str) -> Option<String> {
    let header = mail
        .headers
        .iter()
        .find(|header| header.get_key().eq_ignore_ascii_case(header_name))?;
    let address_list = mailparse::addrparse_header(header).ok()?;
    let mut recipients = Vec::new();
    for address in address_list.iter() {
        flatten_mail_address(address, &mut recipients);
    }
    Some(recipients.join(", "))
}

fn flatten_mail_address(address: &mailparse::MailAddr, recipients: &mut Vec<String>) {
    match address {
        mailparse::MailAddr::Single(single) => {
            if let Some(display_name) = single.display_name.as_ref().filter(|name| !name.is_empty()) {
                recipients.push(format!("{display_name} <{}>", single.addr));
            } else {
                recipients.push(single.addr.clone());
            }
        }
        mailparse::MailAddr::Group(group) => {
            for single in &group.addrs {
                if let Some(display_name) = single.display_name.as_ref().filter(|name| !name.is_empty()) {
                    recipients.push(format!("{display_name} <{}>", single.addr));
                } else {
                    recipients.push(single.addr.clone());
                }
            }
        }
    }
}

async fn imap_delete_message_by_uid(
    account_id: &str,
    folder_id: &str,
    uid: u32,
) -> Result<(), BackendError> {
    let mailbox_name = get_imap_folder_name(account_id, folder_id)
        .ok_or_else(|| BackendError::not_found("IMAP folder not found"))?;
    let mut session = imap_connect_by_account(account_id).await?;
    session
        .select(&mailbox_name)
        .await
        .map_err(|error| BackendError::internal(format!("IMAP SELECT failed: {error}")))?;
    let uid_str = uid.to_string();
    session
        .uid_store(&uid_str, "+FLAGS (\\Deleted)")
        .await
        .map_err(|error| BackendError::internal(format!("IMAP STORE \\Deleted failed: {error}")))?
        .try_collect::<Vec<_>>()
        .await
        .map_err(|error| BackendError::internal(format!("IMAP STORE \\Deleted failed: {error}")))?;
    session
        .expunge()
        .await
        .map_err(|error| BackendError::internal(format!("IMAP EXPUNGE failed: {error}")))?
        .try_collect::<Vec<_>>()
        .await
        .map_err(|error| BackendError::internal(format!("IMAP EXPUNGE failed: {error}")))?;
    let _ = session.logout().await;
    Ok(())
}

fn get_attachment_content_from_storage(
    account_id: &str,
    message_id: &str,
    attachment_id: &str,
) -> Result<AttachmentContent, BackendError> {
    if let Some(content) = memory::get_local_attachment_content(account_id, message_id, attachment_id)?
    {
        return Ok(content);
    }
    let raw = persisted::load_raw_email(message_id).map_err(|_| {
        BackendError::not_found("Raw email not found. Try syncing the account.")
    })?;
    let parsed = mailparse::parse_mail(&raw)
        .map_err(|error| BackendError::internal(format!("Failed to parse email: {error}")))?;
    let part = find_mime_part_by_path(&parsed, attachment_id)
        .ok_or_else(|| BackendError::not_found("Attachment part not found"))?;
    let file_name = get_attachment_filename(part).unwrap_or_else(|| "attachment".into());
    let mime_type = part.ctype.mimetype.clone();
    let raw_body = part
        .get_body_raw()
        .map_err(|error| BackendError::internal(format!("Failed to read attachment body: {error}")))?;
    Ok(AttachmentContent {
        file_name,
        mime_type,
        data_base64: base64_encode_bytes(&raw_body),
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

fn is_gmail_system_label(label: &str) -> bool {
    let lower = label.trim().to_lowercase();
    lower.starts_with("[gmail]/")
        || lower.starts_with("[google mail]/")
        || matches!(
            lower.as_str(),
            "inbox"
                | "sent"
                | "sent mail"
                | "draft"
                | "drafts"
                | "trash"
                | "spam"
                | "junk"
                | "all mail"
                | "starred"
                | "important"
        )
}

fn normalize_gmail_label(label: &str) -> Option<String> {
    let decoded = decode_imap_utf7(label).trim().to_string();
    if decoded.is_empty() || decoded.starts_with('\\') || is_gmail_system_label(&decoded) {
        return None;
    }
    Some(decoded)
}

fn normalize_keyword_label(label: &str) -> Option<String> {
    let trimmed = label.trim();
    if trimmed.is_empty() || trimmed.starts_with('\\') {
        return None;
    }
    Some(trimmed.into())
}

fn validate_imap_keyword_label_name(label: &str) -> Result<String, BackendError> {
    let trimmed = label.trim();
    if trimmed.is_empty() {
        return Err(BackendError::validation("Label name cannot be empty"));
    }
    if !trimmed.is_ascii() {
        return Err(BackendError::validation(
            "This IMAP server only supports ASCII keyword labels",
        ));
    }
    if trimmed.chars().any(|ch| {
        ch.is_whitespace() || matches!(ch, '(' | ')' | '{' | '}' | '%' | '*' | '"' | '\\' | ']')
    }) {
        return Err(BackendError::validation(
            "This IMAP server does not allow this label name",
        ));
    }
    Ok(trimmed.into())
}

async fn imap_account_uses_gmail_labels(account_id: &str) -> Result<bool, BackendError> {
    let mut session = imap_connect_by_account(account_id).await?;
    let capabilities = session
        .capabilities()
        .await
        .map_err(|error| BackendError::internal(format!("IMAP CAPABILITY failed: {error}")))?;
    let _ = session.logout().await;
    Ok(capabilities.has_str("X-GM-EXT-1"))
}

async fn imap_fetch_gmail_labels(
    session: &mut ImapAnySession,
    uid_set: &str,
) -> Result<std::collections::HashMap<u32, Vec<String>>, BackendError> {
    let request_id = session
        .run_command(format!("UID FETCH {uid_set} (UID X-GM-LABELS)"))
        .await
        .map_err(|error| {
            BackendError::internal(format!("IMAP Gmail label fetch failed to send: {error}"))
        })?;

    let mut labels_by_uid = std::collections::HashMap::new();

    loop {
        let response = session.read_response().await.map_err(|error| {
            BackendError::internal(format!("IMAP Gmail label fetch read failed: {error}"))
        })?;
        let Some(response) = response else {
            return Err(BackendError::internal(
                "IMAP Gmail label fetch connection lost",
            ));
        };

        match response.parsed() {
            imap_proto::Response::Fetch(_, attrs) => {
                let mut uid = None;
                let mut labels = Vec::new();
                for attr in attrs {
                    match attr {
                        imap_proto::types::AttributeValue::Uid(value) => uid = Some(*value),
                        imap_proto::types::AttributeValue::GmailLabels(values) => {
                            labels = values
                                .iter()
                                .filter_map(|value| normalize_gmail_label(value.as_ref()))
                                .collect();
                        }
                        _ => {}
                    }
                }
                if let Some(uid) = uid {
                    labels_by_uid.insert(uid, labels);
                }
            }
            imap_proto::Response::Done {
                tag,
                status,
                information,
                ..
            } if tag == &request_id => match status {
                imap_proto::Status::Ok => break,
                other => {
                    return Err(BackendError::internal(format!(
                        "IMAP Gmail label fetch failed: {other:?} {information:?}"
                    )));
                }
            },
            _ => {}
        }
    }

    Ok(labels_by_uid)
}

async fn imap_store_gmail_label(
    session: &mut ImapAnySession,
    uid: u32,
    label: &str,
    add: bool,
) -> Result<(), BackendError> {
    let encoded_label = quote_imap_string(&encode_imap_utf7(label.trim()));
    let operation = if add { "+X-GM-LABELS" } else { "-X-GM-LABELS" };
    session
        .run_command_and_check_ok(format!("UID STORE {uid} {operation} ({encoded_label})"))
        .await
        .map_err(|error| {
            BackendError::internal(format!("IMAP Gmail label store failed: {error}"))
        })?;
    Ok(())
}

async fn imap_store_keyword_label(
    session: &mut ImapAnySession,
    uid: u32,
    label: &str,
    add: bool,
) -> Result<(), BackendError> {
    let keyword = validate_imap_keyword_label_name(label)?;
    let query = if add {
        format!("+FLAGS ({keyword})")
    } else {
        format!("-FLAGS ({keyword})")
    };
    session
        .uid_store(uid.to_string(), &query)
        .await
        .map_err(|error| {
            BackendError::internal(format!("IMAP keyword label store failed: {error}"))
        })?
        .try_collect::<Vec<_>>()
        .await
        .map_err(|error| {
            BackendError::internal(format!("IMAP keyword label store failed: {error}"))
        })?;
    Ok(())
}

async fn imap_store_server_label(
    account_id: &str,
    folder_id: &str,
    uid: u32,
    label: &str,
    add: bool,
) -> Result<(), BackendError> {
    let mailbox_name = get_imap_folder_name(account_id, folder_id)
        .ok_or_else(|| BackendError::internal("IMAP folder name not found"))?;

    let mut session = imap_connect_by_account(account_id).await?;
    let capabilities = session
        .capabilities()
        .await
        .map_err(|error| BackendError::internal(format!("IMAP CAPABILITY failed: {error}")))?;

    session
        .select(&mailbox_name)
        .await
        .map_err(|e| BackendError::internal(format!("IMAP SELECT failed: {e}")))?;

    let result = if capabilities.has_str("X-GM-EXT-1") {
        imap_store_gmail_label(&mut session, uid, label, add).await
    } else {
        imap_store_keyword_label(&mut session, uid, label, add).await
    };

    let _ = session.logout().await;
    result
}

async fn imap_apply_keyword_label_batch(
    account_id: &str,
    label: &str,
    new_label: Option<&str>,
) -> Result<(), BackendError> {
    let source_label = validate_imap_keyword_label_name(label)?;
    let target_label = match new_label {
        Some(label) => Some(validate_imap_keyword_label_name(label)?),
        None => None,
    };

    for folder in memory::list_folders(account_id)? {
        let Some(mailbox_name) = folder.imap_name.clone() else {
            continue;
        };

        let mut session = imap_connect_by_account(account_id).await?;
        session
            .select(&mailbox_name)
            .await
            .map_err(|error| BackendError::internal(format!("IMAP SELECT failed: {error}")))?;

        let search_query = format!("KEYWORD {source_label}");
        let uids = session.uid_search(search_query).await.map_err(|error| {
            BackendError::internal(format!("IMAP keyword search failed: {error}"))
        })?;
        if uids.is_empty() {
            let _ = session.logout().await;
            continue;
        }

        let uid_set = uids
            .into_iter()
            .map(|uid| uid.to_string())
            .collect::<Vec<_>>()
            .join(",");
        if let Some(ref next_label) = target_label {
            let add_query = format!("+FLAGS ({next_label})");
            session
                .uid_store(&uid_set, &add_query)
                .await
                .map_err(|error| {
                    BackendError::internal(format!("IMAP keyword relabel add failed: {error}"))
                })?
                .try_collect::<Vec<_>>()
                .await
                .map_err(|error| {
                    BackendError::internal(format!("IMAP keyword relabel add failed: {error}"))
                })?;
        }

        let remove_query = format!("-FLAGS ({source_label})");
        session
            .uid_store(&uid_set, &remove_query)
            .await
            .map_err(|error| {
                BackendError::internal(format!("IMAP keyword relabel remove failed: {error}"))
            })?
            .try_collect::<Vec<_>>()
            .await
            .map_err(|error| {
                BackendError::internal(format!("IMAP keyword relabel remove failed: {error}"))
            })?;
        let _ = session.logout().await;
    }

    Ok(())
}

async fn imap_rename_keyword_label(
    account_id: &str,
    label: &str,
    new_label: &str,
) -> Result<(), BackendError> {
    imap_apply_keyword_label_batch(account_id, label, Some(new_label)).await
}

async fn imap_delete_keyword_label(account_id: &str, label: &str) -> Result<(), BackendError> {
    imap_apply_keyword_label_batch(account_id, label, None).await
}

async fn imap_rename_gmail_label(
    account_id: &str,
    label: &str,
    new_label: &str,
) -> Result<(), BackendError> {
    let current_name = encode_imap_utf7(label.trim());
    let next_name = encode_imap_utf7(new_label.trim());
    let mut session = imap_connect_by_account(account_id).await?;
    session
        .rename(&current_name, &next_name)
        .await
        .map_err(|error| {
            BackendError::internal(format!("IMAP Gmail label rename failed: {error}"))
        })?;
    let _ = session.logout().await;
    Ok(())
}

async fn imap_delete_gmail_label(account_id: &str, label: &str) -> Result<(), BackendError> {
    let mailbox_name = encode_imap_utf7(label.trim());
    let mut session = imap_connect_by_account(account_id).await?;
    session.delete(&mailbox_name).await.map_err(|error| {
        BackendError::internal(format!("IMAP Gmail label delete failed: {error}"))
    })?;
    let _ = session.logout().await;
    Ok(())
}

async fn imap_list_labels(account_id: &str) -> Result<Vec<MailLabel>, BackendError> {
    let local_labels = memory::list_labels(account_id)?;
    if !imap_account_uses_gmail_labels(account_id).await? {
        return Ok(local_labels);
    }

    let mut counts = local_labels
        .into_iter()
        .map(|label| (label.name.to_lowercase(), label))
        .collect::<std::collections::HashMap<_, _>>();

    let mut session = imap_connect_by_account(account_id).await?;
    let remote_folders = session
        .list(None, Some("*"))
        .await
        .map_err(|error| BackendError::internal(format!("IMAP LIST labels failed: {error}")))?
        .try_collect::<Vec<_>>()
        .await
        .map_err(|error| BackendError::internal(format!("IMAP LIST labels failed: {error}")))?;
    let _ = session.logout().await;

    for remote_folder in remote_folders {
        let name = decode_imap_utf7(remote_folder.name());
        if is_gmail_system_label(&name) {
            continue;
        }
        let key = name.to_lowercase();
        counts.entry(key).or_insert(MailLabel { name, count: 0 });
    }

    let mut labels = counts.into_values().collect::<Vec<_>>();
    labels.sort_by(|left, right| left.name.to_lowercase().cmp(&right.name.to_lowercase()));
    Ok(labels)
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
    eprintln!(
        "[imap] incremental sync: {} cached bodies available",
        existing_bodies.len()
    );

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
        imap_uid_validity: None,
        imap_uid_next: None,
        imap_highest_modseq: None,
    });

    for remote_folder in &remote_folders {
        let raw_name = remote_folder.name();

        if remote_folder
            .attributes()
            .iter()
            .any(|a| matches!(a, async_imap::types::NameAttribute::NoSelect))
        {
            continue;
        }

        let display_name = decode_imap_utf7(raw_name);
        let (kind, icon) = classify_folder(&display_name);
        let folder_id = format!("{}-{}", slug(raw_name), account_id);

        let (unread, total, messages, threads) = fetch_folder_contents(
            &mut session,
            account_id,
            &folder_id,
            raw_name,
            &existing_bodies,
        )
        .await?;

        folders.push(MailboxFolder {
            id: folder_id,
            account_id: account_id.clone(),
            name: display_name,
            kind,
            unread_count: unread,
            total_count: total,
            icon: icon.into(),
            imap_name: Some(raw_name.to_string()),
            imap_uid_validity: None,
            imap_uid_next: None,
            imap_highest_modseq: None,
        });

        all_messages.extend(messages);
        all_threads.extend(threads);
    }

    let _ = session.logout().await;
    Ok((folders, all_messages, all_threads))
}

async fn fetch_folder_contents(
    session: &mut ImapAnySession,
    account_id: &str,
    folder_id: &str,
    mailbox_name: &str,
    existing_bodies: &ExistingBodies,
) -> Result<(u32, u32, Vec<MailMessage>, Vec<MailThread>), BackendError> {
    let result = fetch_folder_contents_incremental(
        session,
        account_id,
        folder_id,
        mailbox_name,
        existing_bodies,
        None,
        50,
    )
    .await?;

    Ok((result.unread, result.total, result.messages, result.threads))
}

async fn fetch_folder_contents_incremental(
    session: &mut ImapAnySession,
    account_id: &str,
    folder_id: &str,
    mailbox_name: &str,
    existing_bodies: &ExistingBodies,
    known_max_uid: Option<u32>,
    recent_limit: u32,
) -> Result<FolderSyncResult, BackendError> {
    let previous_folder = memory::list_folders(account_id)
        .ok()
        .and_then(|folders| folders.into_iter().find(|folder| folder.id == folder_id));
    let previous_uid_validity = previous_folder
        .as_ref()
        .and_then(|folder| folder.imap_uid_validity);
    let previous_highest_modseq = previous_folder
        .as_ref()
        .and_then(|folder| folder.imap_highest_modseq);
    let select_result = select_mailbox_for_incremental_sync(
        session,
        account_id,
        folder_id,
        mailbox_name,
        previous_folder.as_ref(),
    )
    .await?;
    let mailbox = select_result.mailbox;

    let total = mailbox.exists;
    let uid_validity = mailbox.uid_validity;
    let uid_next = mailbox.uid_next;
    let highest_modseq = mailbox.highest_modseq;
    let uid_validity_changed = previous_uid_validity.is_some()
        && uid_validity.is_some()
        && previous_uid_validity != uid_validity;
    let use_changedsince =
        !uid_validity_changed && previous_highest_modseq.is_some() && highest_modseq.is_some();

    if total == 0 {
        return Ok(FolderSyncResult {
            unread: 0,
            total,
            uid_validity,
            uid_next,
            highest_modseq,
            vanished_uids: select_result.vanished_uids,
            messages: Vec::new(),
            threads: Vec::new(),
            fetched_all_messages: true,
        });
    }

    // SEARCH UNSEEN to get the actual unread count (SELECT's UNSEEN is just
    // the sequence number of the first unseen message, not the count).
    let unread = session
        .search("UNSEEN")
        .await
        .map(|ids| ids.len() as u32)
        .unwrap_or(0);

    let capabilities = session
        .capabilities()
        .await
        .map_err(|error| BackendError::internal(format!("IMAP CAPABILITY failed: {error}")))?;
    let has_gmail_labels = capabilities.has_str("X-GM-EXT-1");

    // Always refresh the recent window for flags/read state, and fetch any UID newer
    // than the local max so realtime sync does not rescan the whole mailbox.
    let start = if total > recent_limit {
        total - (recent_limit - 1)
    } else {
        1
    };
    let range = format!("{start}:{total}");

    let recent_query = if let Some(modseq) = previous_highest_modseq.filter(|_| use_changedsince) {
        let attrs = if has_gmail_labels {
            "UID FLAGS ENVELOPE RFC822.SIZE MODSEQ X-GM-LABELS"
        } else {
            "UID FLAGS ENVELOPE RFC822.SIZE MODSEQ"
        };
        format!("({attrs}) (CHANGEDSINCE {modseq})")
    } else {
        if has_gmail_labels {
            "(UID FLAGS ENVELOPE RFC822.SIZE MODSEQ X-GM-LABELS)".into()
        } else {
            "(UID FLAGS ENVELOPE RFC822.SIZE MODSEQ)".into()
        }
    };
    let recent_fetches: Vec<_> = session
        .fetch(&range, &recent_query)
        .await
        .map_err(|e| BackendError::internal(format!("IMAP FETCH headers failed: {e}")))?
        .try_collect()
        .await
        .map_err(|e| BackendError::internal(format!("IMAP FETCH headers failed: {e}")))?;

    let mut fetches = recent_fetches;
    if let Some(max_uid) = known_max_uid {
        let next_uid = max_uid.saturating_add(1);
        let uid_range = format!("{next_uid}:*");
        let uid_query = if has_gmail_labels {
            "(UID FLAGS ENVELOPE RFC822.SIZE MODSEQ X-GM-LABELS)"
        } else {
            "(UID FLAGS ENVELOPE RFC822.SIZE MODSEQ)"
        };
        if let Ok(stream) = session.uid_fetch(&uid_range, uid_query).await {
            if let Ok(new_fetches) = stream.try_collect::<Vec<_>>().await {
                let mut seen_uids: std::collections::HashSet<u32> = fetches
                    .iter()
                    .map(|fetch| fetch.uid.unwrap_or(fetch.message))
                    .collect();
                for fetch in new_fetches {
                    let uid = fetch.uid.unwrap_or(fetch.message);
                    if seen_uids.insert(uid) {
                        fetches.push(fetch);
                    }
                }
            }
        }
    }

    let mut metas: Vec<MsgMeta> = Vec::with_capacity(fetches.len());
    let mut new_uids: Vec<u32> = Vec::new();
    let mut labels_by_uid = std::collections::HashMap::new();

    if has_gmail_labels && !fetches.is_empty() {
        let uid_set = fetches
            .iter()
            .map(|fetch| fetch.uid.unwrap_or(fetch.message).to_string())
            .collect::<Vec<_>>()
            .join(",");
        labels_by_uid = imap_fetch_gmail_labels(session, &uid_set).await?;
    }

    for fetch in &fetches {
        let uid = fetch.uid.unwrap_or(fetch.message);
        let message_id = format!("imap-{account_id}-{folder_id}-{uid}");
        let thread_id = format!("thread-{message_id}");

        let is_read = fetch
            .flags()
            .any(|f| matches!(f, async_imap::types::Flag::Seen));
        let is_starred = fetch
            .flags()
            .any(|f| matches!(f, async_imap::types::Flag::Flagged));
        let labels = if has_gmail_labels {
            labels_by_uid.get(&uid).cloned().unwrap_or_default()
        } else {
            fetch
                .flags()
                .filter_map(|flag| match flag {
                    async_imap::types::Flag::Custom(value) => {
                        normalize_keyword_label(value.as_ref())
                    }
                    _ => None,
                })
                .collect()
        };

        let (subject, from, from_email, to, cc, date) = match fetch.envelope() {
            Some(env) => {
                let (subj, frm, frm_email, t, c, d) = parse_envelope(env);
                // If envelope has no date, log a warning
                if d.is_empty() {
                    eprintln!(
                        "[imap] WARNING: no date in envelope for '{}' from {}",
                        subj, frm
                    );
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
            uid,
            message_id,
            thread_id,
            is_read,
            is_starred,
            subject,
            from,
            from_email,
            to,
            cc,
            date,
            labels,
        });
    }

    // --- Phase 2: fetch full body ONLY for new messages ---
    let mut body_map: std::collections::HashMap<u32, Vec<u8>> = std::collections::HashMap::new();

    if !new_uids.is_empty() {
        eprintln!(
            "[imap] fetching body for {} new messages in {mailbox_name}",
            new_uids.len()
        );
        let uid_set = new_uids
            .iter()
            .map(|u| u.to_string())
            .collect::<Vec<_>>()
            .join(",");
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
        eprintln!(
            "[imap] all {} messages cached in {mailbox_name}, skipping body fetch",
            metas.len()
        );
    }

    // --- Phase 3: assemble messages ---
    let mut messages = Vec::with_capacity(metas.len());
    let mut threads = Vec::with_capacity(metas.len());

    for meta in metas {
        let (body, preview, attachments, final_date) = if meta.date.is_empty() {
            // If no date in envelope, check if we have cached data with a valid date
            if let Some((cached_body, cached_preview, cached_attachments, cached_date)) =
                existing_bodies.get(&meta.uid)
            {
                // Use cached data if we have a valid date (not current timestamp pattern)
                if !cached_date.is_empty() && !cached_date.starts_with("2026-03-16") {
                    // Cached date looks valid, use it
                    (
                        cached_body.clone(),
                        cached_preview.clone(),
                        cached_attachments.clone(),
                        cached_date.clone(),
                    )
                } else if let Some(raw) = body_map.remove(&meta.uid) {
                    // Need to extract date from body
                    let parsed = extract_body_from_mime(&raw);
                    let prev = make_preview(&parsed);
                    let atts = extract_attachments_from_mime(&raw);
                    let date = extract_date_from_body(&raw, meta.uid);
                    (parsed, prev, atts, date)
                } else {
                    // No body available, use cached data with current time
                    eprintln!(
                        "[imap] WARNING: no body available for UID {} with empty date",
                        meta.uid
                    );
                    (
                        cached_body.clone(),
                        cached_preview.clone(),
                        cached_attachments.clone(),
                        memory::current_timestamp(),
                    )
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
                eprintln!(
                    "[imap] WARNING: no body available for UID {} with empty date",
                    meta.uid
                );
                (
                    String::new(),
                    "(No preview)".into(),
                    vec![],
                    memory::current_timestamp(),
                )
            }
        } else if let Some((cached_body, cached_preview, cached_attachments, _)) =
            existing_bodies.get(&meta.uid)
        {
            // Reuse body + preview + attachments from memory, use envelope date
            (
                cached_body.clone(),
                cached_preview.clone(),
                cached_attachments.clone(),
                meta.date.clone(),
            )
        } else if let Some(raw) = body_map.remove(&meta.uid) {
            // Parse newly downloaded body and extract attachments
            let parsed = extract_body_from_mime(&raw);
            let prev = make_preview(&parsed);
            let atts = extract_attachments_from_mime(&raw);
            (parsed, prev, atts, meta.date.clone())
        } else {
            (
                String::new(),
                "(No preview)".into(),
                vec![],
                meta.date.clone(),
            )
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
            labels: meta.labels,
            imap_uid: Some(meta.uid),
            previous_folder_id: None,
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

    Ok(FolderSyncResult {
        unread,
        total,
        uid_validity,
        uid_next,
        highest_modseq,
        vanished_uids: select_result.vanished_uids,
        messages,
        threads,
        fetched_all_messages: uid_validity_changed || (!use_changedsince && total <= recent_limit),
    })
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
    } else if lower.contains("archive")
        || lower.contains("all mail")
        || lower == "[gmail]/all mail"
        || lower.contains("归档")
    {
        (MailFolderKind::Archive, "mdi-archive-outline")
    } else if lower.contains("spam")
        || lower.contains("junk")
        || lower.contains("bulk")
        || lower.contains("unsolicited")
        || lower.contains("垃圾")
        || lower.contains("拦截")
    {
        (MailFolderKind::Junk, "mdi-alert-circle-outline")
    } else {
        (MailFolderKind::Custom, "mdi-folder-outline")
    }
}

async fn select_mailbox_for_incremental_sync(
    session: &mut ImapAnySession,
    account_id: &str,
    folder_id: &str,
    mailbox_name: &str,
    previous_folder: Option<&MailboxFolder>,
) -> Result<MailboxSelectResult, BackendError> {
    let capabilities = session
        .capabilities()
        .await
        .map_err(|error| BackendError::internal(format!("IMAP CAPABILITY failed: {error}")))?;
    let has_qresync = capabilities.has_str("QRESYNC");
    let has_condstore = has_qresync || capabilities.has_str("CONDSTORE");
    eprintln!(
        "[imap] mailbox sync capabilities account={} mailbox={} qresync={} condstore={}",
        account_id, mailbox_name, has_qresync, has_condstore
    );

    if has_qresync {
        match session.run_command_and_check_ok("ENABLE QRESYNC").await {
            Ok(()) => {
                eprintln!(
                    "[imap] enabled QRESYNC account={} mailbox={}",
                    account_id, mailbox_name
                );
            }
            Err(error) => {
                eprintln!(
                    "[imap] ENABLE QRESYNC failed account={} mailbox={}: {}",
                    account_id, mailbox_name, error
                );
            }
        }
    }

    if has_qresync {
        if let Some(folder) = previous_folder {
            if let (Some(uid_validity), Some(highest_modseq)) =
                (folder.imap_uid_validity, folder.imap_highest_modseq)
            {
                let known_uids = memory::get_imap_uids_for_folder(account_id, folder_id);
                if !known_uids.is_empty() {
                    eprintln!(
                        "[imap] attempting QRESYNC SELECT account={} mailbox={} uidValidity={} modseq={} knownUids={}",
                        account_id,
                        mailbox_name,
                        uid_validity,
                        highest_modseq,
                        known_uids.len()
                    );
                    match qresync_select_mailbox(
                        session,
                        mailbox_name,
                        uid_validity,
                        highest_modseq,
                        &known_uids,
                    )
                    .await
                    {
                        Ok(result) => {
                            eprintln!(
                                "[imap] QRESYNC SELECT ok account={} mailbox={} vanished={}",
                                account_id,
                                mailbox_name,
                                result.vanished_uids.len()
                            );
                            return Ok(result);
                        }
                        Err(error) => {
                            eprintln!(
                                "[imap] QRESYNC SELECT failed, falling back account={} mailbox={}: {}",
                                account_id,
                                mailbox_name,
                                error.message
                            );
                        }
                    }
                } else {
                    eprintln!(
                        "[imap] skipping QRESYNC SELECT account={} mailbox={} reason=no-known-uids",
                        account_id, mailbox_name
                    );
                }
            } else {
                eprintln!(
                    "[imap] skipping QRESYNC SELECT account={} mailbox={} reason=missing-baseline uidValidity={:?} modseq={:?}",
                    account_id,
                    mailbox_name,
                    folder.imap_uid_validity,
                    folder.imap_highest_modseq
                );
            }
        } else {
            eprintln!(
                "[imap] skipping QRESYNC SELECT account={} mailbox={} reason=no-local-folder-state",
                account_id, mailbox_name
            );
        }
    }

    if has_condstore {
        eprintln!(
            "[imap] falling back to CONDSTORE SELECT account={} mailbox={}",
            account_id, mailbox_name
        );
        let mailbox = session
            .select_condstore(mailbox_name)
            .await
            .map_err(|error| {
                BackendError::internal(format!(
                    "IMAP SELECT CONDSTORE '{mailbox_name}' failed: {error}"
                ))
            })?;
        return Ok(MailboxSelectResult {
            mailbox,
            vanished_uids: Vec::new(),
        });
    }

    eprintln!(
        "[imap] falling back to plain SELECT account={} mailbox={}",
        account_id, mailbox_name
    );
    let mailbox = session.select(mailbox_name).await.map_err(|error| {
        BackendError::internal(format!("IMAP SELECT '{mailbox_name}' failed: {error}"))
    })?;
    Ok(MailboxSelectResult {
        mailbox,
        vanished_uids: Vec::new(),
    })
}

fn resolve_or_create_folder(account_id: &str, mailbox_name: &str) -> MailboxFolder {
    if let Ok(folders) = memory::list_folders(account_id) {
        if let Some(folder) = folders
            .into_iter()
            .find(|folder| folder.imap_name.as_deref() == Some(mailbox_name))
        {
            return folder;
        }
    }

    let display_name = decode_imap_utf7(mailbox_name);
    let (kind, icon) = classify_folder(&display_name);
    MailboxFolder {
        id: format!("{}-{}", slug(mailbox_name), account_id),
        account_id: account_id.into(),
        name: display_name,
        kind,
        unread_count: 0,
        total_count: 0,
        icon: icon.into(),
        imap_name: Some(mailbox_name.into()),
        imap_uid_validity: None,
        imap_uid_next: None,
        imap_highest_modseq: None,
    }
}

async fn qresync_select_mailbox(
    session: &mut ImapAnySession,
    mailbox_name: &str,
    uid_validity: u32,
    highest_modseq: u64,
    known_uids: &[u32],
) -> Result<MailboxSelectResult, BackendError> {
    let known_uid_set = compress_uid_set(known_uids);
    let command = format!(
        "SELECT {} (QRESYNC ({} {} {}))",
        quote_imap_string(mailbox_name),
        uid_validity,
        highest_modseq,
        known_uid_set,
    );
    let request_id = session.run_command(&command).await.map_err(|error| {
        BackendError::internal(format!("IMAP QRESYNC SELECT failed to send: {error}"))
    })?;

    let mut mailbox = async_imap::types::Mailbox::default();
    let mut vanished_uids = Vec::new();

    loop {
        let response = session.read_response().await.map_err(|error| {
            BackendError::internal(format!("IMAP QRESYNC SELECT read failed: {error}"))
        })?;
        let Some(response) = response else {
            return Err(BackendError::internal(
                "IMAP QRESYNC SELECT connection lost",
            ));
        };

        match response.parsed() {
            imap_proto::Response::Done {
                tag,
                status,
                code,
                information,
                ..
            } if tag == &request_id => match status {
                imap_proto::Status::Ok => break,
                imap_proto::Status::Bad | imap_proto::Status::No => {
                    return Err(BackendError::internal(format!(
                        "IMAP QRESYNC SELECT rejected: code={code:?} info={information:?}"
                    )));
                }
                other => {
                    return Err(BackendError::internal(format!(
                        "IMAP QRESYNC SELECT unexpected status: {other:?}"
                    )));
                }
            },
            imap_proto::Response::Data {
                status: imap_proto::Status::Ok,
                code,
                ..
            } => apply_mailbox_code(&mut mailbox, code.as_ref()),
            imap_proto::Response::MailboxData(data) => match data {
                imap_proto::MailboxDatum::Exists(exists) => mailbox.exists = *exists,
                imap_proto::MailboxDatum::Recent(recent) => mailbox.recent = *recent,
                imap_proto::MailboxDatum::Flags(flags) => {
                    mailbox.flags.extend(
                        flags
                            .iter()
                            .map(|flag| (*flag).to_string())
                            .map(async_imap::types::Flag::from),
                    );
                }
                _ => {}
            },
            imap_proto::Response::Vanished { uids, .. } => {
                vanished_uids.extend(uids.iter().flat_map(|range| range.clone()));
            }
            _ => {}
        }
    }

    Ok(MailboxSelectResult {
        mailbox,
        vanished_uids,
    })
}

fn apply_mailbox_code(
    mailbox: &mut async_imap::types::Mailbox,
    code: Option<&imap_proto::ResponseCode<'_>>,
) {
    match code {
        Some(imap_proto::ResponseCode::UidValidity(uid)) => mailbox.uid_validity = Some(*uid),
        Some(imap_proto::ResponseCode::UidNext(uid_next)) => mailbox.uid_next = Some(*uid_next),
        Some(imap_proto::ResponseCode::HighestModSeq(modseq)) => {
            mailbox.highest_modseq = Some(*modseq)
        }
        Some(imap_proto::ResponseCode::Unseen(unseen)) => mailbox.unseen = Some(*unseen),
        Some(imap_proto::ResponseCode::PermanentFlags(flags)) => {
            mailbox.permanent_flags.extend(
                flags
                    .iter()
                    .map(|flag| (*flag).to_string())
                    .map(async_imap::types::Flag::from),
            );
        }
        _ => {}
    }
}

fn compress_uid_set(uids: &[u32]) -> String {
    let mut parts = Vec::new();
    let mut iter = uids.iter().copied();
    let Some(mut start) = iter.next() else {
        return "1".into();
    };
    let mut end = start;

    for uid in iter {
        if uid == end.saturating_add(1) {
            end = uid;
            continue;
        }

        parts.push(if start == end {
            start.to_string()
        } else {
            format!("{start}:{end}")
        });
        start = uid;
        end = uid;
    }

    parts.push(if start == end {
        start.to_string()
    } else {
        format!("{start}:{end}")
    });

    parts.join(",")
}

fn quote_imap_string(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
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

fn encode_imap_utf7(input: &str) -> String {
    let mut result = String::new();
    let mut non_ascii = String::new();

    let flush_non_ascii = |result: &mut String, non_ascii: &mut String| {
        if non_ascii.is_empty() {
            return;
        }

        let utf16 = non_ascii.encode_utf16().collect::<Vec<_>>();
        let mut bytes = Vec::with_capacity(utf16.len() * 2);
        for code_unit in utf16 {
            bytes.extend_from_slice(&code_unit.to_be_bytes());
        }

        let encoded = general_purpose::STANDARD.encode(bytes);
        let encoded = encoded.trim_end_matches('=').replace('/', ",");
        result.push('&');
        result.push_str(&encoded);
        result.push('-');
        non_ascii.clear();
    };

    for character in input.chars() {
        if character == '&' {
            flush_non_ascii(&mut result, &mut non_ascii);
            result.push_str("&-");
        } else if character.is_ascii() && matches!(character, ' '..='~') {
            flush_non_ascii(&mut result, &mut non_ascii);
            result.push(character);
        } else {
            non_ascii.push(character);
        }
    }

    flush_non_ascii(&mut result, &mut non_ascii);
    result
}

fn decode_modified_base64(input: &[u8]) -> Vec<u16> {
    let mut input_str = String::from_utf8_lossy(input).to_string();
    input_str = input_str.replace(',', "/");

    // Pad to multiple of 4
    while !input_str.len().is_multiple_of(4) {
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
            let date_header = mail
                .headers
                .iter()
                .find(|h| h.get_key().eq_ignore_ascii_case("Date"))
                .map(|h| h.get_value());

            if let Some(date_str) = date_header {
                if let Ok(ts) = mailparse::dateparse(&date_str) {
                    let dt =
                        chrono::DateTime::from_timestamp(ts, 0).unwrap_or_else(chrono::Utc::now);
                    return dt.to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
                }
            }

            // No Date header, try to extract from first Received header
            let received_header = mail
                .headers
                .iter()
                .find(|h| h.get_key().eq_ignore_ascii_case("Received"))
                .map(|h| h.get_value());

            if let Some(received_str) = received_header {
                // Received header format: "... ; date"
                if let Some(date_part) = received_str.split(';').next_back() {
                    let date_part = date_part.trim();
                    if let Ok(ts) = mailparse::dateparse(date_part) {
                        let dt = chrono::DateTime::from_timestamp(ts, 0)
                            .unwrap_or_else(chrono::Utc::now);
                        return dt.to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
                    }
                }
            }

            memory::current_timestamp()
        }
        Err(_) => memory::current_timestamp(),
    }
}

fn parse_envelope(
    env: &imap_proto::types::Envelope,
) -> (String, String, String, Vec<String>, Vec<String>, String) {
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
            let mailbox = addr
                .mailbox
                .as_ref()
                .map(|m| String::from_utf8_lossy(m).to_string())
                .unwrap_or_default();
            let host = addr
                .host
                .as_ref()
                .map(|h| String::from_utf8_lossy(h).to_string())
                .unwrap_or_default();
            let email = format!("{mailbox}@{host}");
            let display = if name.is_empty() { email.clone() } else { name };
            (display, email)
        })
        .unwrap_or_else(|| ("Unknown".into(), "unknown@unknown".into()));

    let extract_addresses = |addrs: &[imap_proto::types::Address]| -> Vec<String> {
        addrs
            .iter()
            .map(|addr| {
                let mailbox = addr
                    .mailbox
                    .as_ref()
                    .map(|m| String::from_utf8_lossy(m).to_string())
                    .unwrap_or_default();
                let host = addr
                    .host
                    .as_ref()
                    .map(|h| String::from_utf8_lossy(h).to_string())
                    .unwrap_or_default();
                format!("{mailbox}@{host}")
            })
            .collect()
    };

    let to = env
        .to
        .as_ref()
        .map(|a| extract_addresses(a))
        .unwrap_or_default();
    let cc = env
        .cc
        .as_ref()
        .map(|a| extract_addresses(a))
        .unwrap_or_default();

    let date = env
        .date
        .as_ref()
        .map(|d| {
            let raw = String::from_utf8_lossy(d).to_string();
            // Try to parse RFC 2822 date and convert to ISO 8601
            mailparse::dateparse(&raw)
                .map(|ts| {
                    let secs = ts;
                    let dt =
                        chrono::DateTime::from_timestamp(secs, 0).unwrap_or_else(chrono::Utc::now);
                    dt.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
                })
                .unwrap_or_else(|e| {
                    eprintln!("[imap] WARNING: failed to parse date '{}': {:?}", raw, e);
                    String::new() // Return empty string instead of current time
                })
        })
        .unwrap_or_default(); // Return empty string if no date field

    (subject, from, from_email, to, cc, date)
}

// SMTP helper (async)
// ---------------------------------------------------------------------------

pub async fn smtp_send(
    state: &StoredAccountState,
    draft: &DraftMessage,
) -> Result<Vec<u8>, BackendError> {
    use lettre::transport::smtp::authentication::{Credentials, Mechanism};
    use lettre::{AsyncSmtpTransport, AsyncTransport, Tokio1Executor};

    let email = build_rfc822_message(state, draft)?;
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

fn build_rfc822_message(
    state: &StoredAccountState,
    draft: &DraftMessage,
) -> Result<lettre::Message, BackendError> {
    use lettre::message::header::ContentType;
    use lettre::message::header::{HeaderName, HeaderValue, InReplyTo};
    use lettre::message::{Attachment, Mailbox, MultiPart, SinglePart};
    use lettre::Message;

    let identity = resolve_sender_identity(&state.account, draft.selected_identity_id.as_deref());
    let from: Mailbox = format!("{} <{}>", identity.name, identity.email)
        .parse()
        .map_err(|e| BackendError::validation(format!("Invalid sender address: {e}")))?;

    let mut builder = Message::builder().from(from);
    if let Some(identity_id) = draft
        .selected_identity_id
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        builder = builder.raw_header(HeaderValue::new(
            HeaderName::new_from_ascii_str(DRAFT_HEADER_IDENTITY),
            identity_id.clone(),
        ));
    }
    if let Some(reply_id) = draft
        .in_reply_to_message_id
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        builder = builder
            .header(InReplyTo::from(reply_id.clone()))
            .raw_header(HeaderValue::new(
                HeaderName::new_from_ascii_str(DRAFT_HEADER_REPLY),
                reply_id.clone(),
            ));
    }
    if let Some(forward_id) = draft
        .forward_from_message_id
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        builder = builder.raw_header(HeaderValue::new(
            HeaderName::new_from_ascii_str(DRAFT_HEADER_FORWARD),
            forward_id.clone(),
        ));
    }
    if !draft.bcc.trim().is_empty() {
        builder = builder.raw_header(HeaderValue::new(
            HeaderName::new_from_ascii_str(DRAFT_HEADER_BCC),
            draft.bcc.clone(),
        ));
    }
    if let Some(reply_to) = identity
        .reply_to
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        let reply_to_mailbox: Mailbox = reply_to
            .parse()
            .map_err(|e| BackendError::validation(format!("Invalid Reply-To address: {e}")))?;
        builder = builder.reply_to(reply_to_mailbox);
    }

    for recipient in draft.to.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        let to: Mailbox = recipient.parse().map_err(|e| {
            BackendError::validation(format!("Invalid recipient '{recipient}': {e}"))
        })?;
        builder = builder.to(to);
    }

    for recipient in draft.cc.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        let cc: Mailbox = recipient
            .parse()
            .map_err(|e| BackendError::validation(format!("Invalid CC '{recipient}': {e}")))?;
        builder = builder.cc(cc);
    }

    for recipient in draft
        .bcc
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
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
            let content_type =
                ContentType::parse(&att.mime_type).unwrap_or(ContentType::TEXT_PLAIN);
            let attachment = Attachment::new(att.file_name.clone()).body(decoded, content_type);
            multipart = multipart.singlepart(attachment);
        }

        builder
            .subject(&draft.subject)
            .multipart(multipart)
            .map_err(|e| BackendError::internal(format!("Failed to build email: {e}")))?
    };

    Ok(email)
}

fn resolve_sender_identity(account: &MailAccount, identity_id: Option<&str>) -> MailIdentity {
    if let Some(identity_id) = identity_id {
        if let Some(identity) = account
            .identities
            .iter()
            .find(|identity| identity.id == identity_id)
        {
            return identity.clone();
        }
    }

    account
        .identities
        .iter()
        .find(|identity| identity.is_default)
        .cloned()
        .or_else(|| account.identities.first().cloned())
        .unwrap_or(MailIdentity {
            id: format!("identity-{}-default", account.id),
            name: account.name.clone(),
            email: account.email.clone(),
            reply_to: None,
            signature: None,
            is_default: true,
        })
}

#[cfg(test)]
mod tests {
    use super::{
        build_rfc822_message, parse_address_header, parse_text_header, DRAFT_HEADER_BCC,
        DRAFT_HEADER_FORWARD, DRAFT_HEADER_IDENTITY, DRAFT_HEADER_REPLY,
    };
    use crate::models::{
        AccountAuthMode, AccountConfig, AccountStatus, DraftAttachment, DraftMessage, MailAccount,
        MailIdentity, StoredAccountState,
    };

    fn fixture_state() -> StoredAccountState {
        StoredAccountState {
            account: MailAccount {
                id: "acc-1".into(),
                name: "Primary".into(),
                email: "primary@example.com".into(),
                provider: "imap-smtp".into(),
                incoming_protocol: "imap".into(),
                auth_mode: AccountAuthMode::Password,
                oauth_provider: None,
                oauth_source: None,
                color: "#5B8DEF".into(),
                initials: "PR".into(),
                unread_count: 0,
                status: AccountStatus::Connected,
                last_synced_at: "2026-03-17T00:00:00.000Z".into(),
                identities: vec![
                    MailIdentity {
                        id: "identity-default".into(),
                        name: "Primary".into(),
                        email: "primary@example.com".into(),
                        reply_to: None,
                        signature: None,
                        is_default: true,
                    },
                    MailIdentity {
                        id: "identity-alt".into(),
                        name: "Alt Sender".into(),
                        email: "alias@example.com".into(),
                        reply_to: Some("reply@example.com".into()),
                        signature: Some("<p>sig</p>".into()),
                        is_default: false,
                    },
                ],
            },
            config: AccountConfig {
                auth_mode: AccountAuthMode::Password,
                incoming_protocol: "imap".into(),
                incoming_host: "imap.example.com".into(),
                incoming_port: 993,
                outgoing_host: "smtp.example.com".into(),
                outgoing_port: 465,
                username: "primary@example.com".into(),
                password: "secret".into(),
                use_tls: true,
                oauth_provider: None,
                oauth_source: None,
                access_token: String::new(),
                refresh_token: String::new(),
                token_expires_at: String::new(),
            },
        }
    }

    fn fixture_draft() -> DraftMessage {
        DraftMessage {
            id: "draft-1".into(),
            account_id: "acc-1".into(),
            selected_identity_id: Some("identity-alt".into()),
            to: "Alice <alice@example.com>".into(),
            cc: "Bob <bob@example.com>".into(),
            bcc: "Secret <secret@example.com>".into(),
            subject: "Subject".into(),
            body: "<p>Hello</p>".into(),
            in_reply_to_message_id: Some("<reply-id@example.com>".into()),
            forward_from_message_id: Some("forward-source-1".into()),
            attachments: vec![DraftAttachment {
                file_name: "hello.txt".into(),
                mime_type: "text/plain".into(),
                data_base64: "aGVsbG8=".into(),
            }],
        }
    }

    #[test]
    fn build_rfc822_message_embeds_mailyou_draft_headers() {
        let state = fixture_state();
        let draft = fixture_draft();
        let message = build_rfc822_message(&state, &draft).expect("message should build");
        let raw = message.formatted();
        let parsed = mailparse::parse_mail(&raw).expect("message should parse");

        assert_eq!(
            parse_text_header(&parsed, DRAFT_HEADER_IDENTITY).as_deref(),
            Some("identity-alt")
        );
        assert_eq!(
            parse_text_header(&parsed, DRAFT_HEADER_REPLY).as_deref(),
            Some("<reply-id@example.com>")
        );
        assert_eq!(
            parse_text_header(&parsed, DRAFT_HEADER_FORWARD).as_deref(),
            Some("forward-source-1")
        );
        assert_eq!(
            parse_text_header(&parsed, "in-reply-to").as_deref(),
            Some("<reply-id@example.com>")
        );
    }

    #[test]
    fn build_rfc822_message_preserves_bcc_header_for_remote_draft_restore() {
        let state = fixture_state();
        let draft = fixture_draft();
        let message = build_rfc822_message(&state, &draft).expect("message should build");
        let raw = message.formatted();
        let parsed = mailparse::parse_mail(&raw).expect("message should parse");

        assert_eq!(
            parse_text_header(&parsed, DRAFT_HEADER_BCC).as_deref(),
            Some("Secret <secret@example.com>")
        );
        assert_eq!(
            parse_address_header(&parsed, "to").as_deref(),
            Some("Alice <alice@example.com>")
        );
        assert_eq!(
            parse_address_header(&parsed, "cc").as_deref(),
            Some("Bob <bob@example.com>")
        );
    }

    #[test]
    fn build_rfc822_message_uses_selected_identity_and_reply_to() {
        let state = fixture_state();
        let draft = fixture_draft();
        let message = build_rfc822_message(&state, &draft).expect("message should build");
        let raw = message.formatted();
        let parsed = mailparse::parse_mail(&raw).expect("message should parse");

        assert_eq!(
            parse_address_header(&parsed, "from").as_deref(),
            Some("Alt Sender <alias@example.com>")
        );
        assert_eq!(
            parse_address_header(&parsed, "reply-to").as_deref(),
            Some("reply@example.com")
        );
    }
}
