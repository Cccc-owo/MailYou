use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

mod client_ops;
mod draft_ops;
mod folder_ops;
mod label_ops;
mod message_ops;
mod parse_ops;
mod smtp_ops;
mod sync_ops;

use async_imap::{Authenticator, Session as ImapSession};
use async_trait::async_trait;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::TcpStream;
use tokio_native_tls::TlsStream;

use crate::models::{
    AccountQuota, AccountSetupDraft, AccountUnreadSnapshot, AttachmentContent, AttachmentMeta,
    DraftMessage, MailAccount, MailLabel, MailMessage, MailThread, MailboxBundle, MailboxFolder,
    StoredAccountState, SyncStatus,
};
use crate::protocol::BackendError;
use crate::provider::common::{
    extract_attachments_from_mime, extract_body_from_mime, make_preview, validate_draft,
};
use crate::provider::{
    AccountProvider, DraftProvider, FolderProvider, LabelProvider, MessageMutationProvider,
    MessageQueryProvider, SyncProvider,
};
use crate::storage::memory;

const TCP_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

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
impl AccountProvider for ImapSmtpProvider {
    async fn list_accounts_cap(&self) -> Result<Vec<MailAccount>, BackendError> {
        memory::store().accounts().list_accounts()
    }

    async fn create_account_cap(&self, draft: AccountSetupDraft) -> Result<MailAccount, BackendError> {
        eprintln!(
            "[imap] testing connection for new account {}...",
            draft.email
        );
        self.test_account_connection_cap(draft.clone()).await?;
        eprintln!("[imap] connection test passed, creating account");
        memory::store().accounts().create_account_without_test(draft)
    }

    async fn test_account_connection_cap(
        &self,
        draft: AccountSetupDraft,
    ) -> Result<SyncStatus, BackendError> {
        validate_draft(&draft)?;
        eprintln!(
            "[imap] connecting to {}:{}...",
            draft.incoming_host, draft.incoming_port
        );
        let start = Instant::now();
        client_ops::imap_login_test(&draft).await?;
        eprintln!("[imap] connection test ok ({:.1?})", start.elapsed());

        Ok(crate::provider::common::build_connection_test_status(format!(
                "Connected to {}:{} and {}:{}",
                draft.incoming_host, draft.incoming_port, draft.outgoing_host, draft.outgoing_port
            )))
    }

    async fn delete_account_cap(&self, account_id: &str) -> Result<(), BackendError> {
        eprintln!("[store] deleting account {account_id}");
        memory::store().accounts().delete_account(account_id)
    }

    async fn get_account_config_cap(
        &self,
        account_id: &str,
    ) -> Result<AccountSetupDraft, BackendError> {
        memory::store().accounts().get_account_config(account_id)
    }

    async fn update_account_cap(
        &self,
        account_id: &str,
        draft: AccountSetupDraft,
    ) -> Result<MailAccount, BackendError> {
        memory::store().accounts().update_account(account_id, draft)
    }

    async fn get_account_quota_cap(
        &self,
        account_id: &str,
    ) -> Result<Option<AccountQuota>, BackendError> {
        let mut session = client_ops::imap_connect_by_account(account_id).await?;
        let quota_result = session.get_quota_root("INBOX").await;
        let _ = session.logout().await;

        let (_quota_roots, quotas) = match quota_result {
            Ok(result) => result,
            Err(error) => {
                eprintln!("[imap] quota unavailable for {account_id}: {error}");
                return Ok(None);
            }
        };

        for quota in quotas {
            let quota_root = quota.root_name;
            for resource in quota.resources {
                if matches!(&resource.name, async_imap::types::QuotaResourceName::Storage) {
                    return Ok(Some(AccountQuota {
                        account_id: account_id.to_string(),
                        quota_root: Some(quota_root),
                        storage_used_kb: Some(resource.usage),
                        storage_limit_kb: Some(resource.limit),
                        usage_percent: Some(resource.get_usage_percentage()),
                        updated_at: memory::current_timestamp(),
                    }));
                }
            }
        }

        Ok(None)
    }
}

#[async_trait]
impl FolderProvider for ImapSmtpProvider {
    async fn list_folders_cap(&self, account_id: &str) -> Result<Vec<MailboxFolder>, BackendError> {
        memory::store().mail().list_folders(account_id)
    }

    async fn create_folder_cap(
        &self,
        account_id: &str,
        name: &str,
    ) -> Result<Vec<MailboxFolder>, BackendError> {
        folder_ops::create_folder(self, account_id, name).await
    }

    async fn rename_folder_cap(
        &self,
        account_id: &str,
        folder_id: &str,
        name: &str,
    ) -> Result<Vec<MailboxFolder>, BackendError> {
        folder_ops::rename_folder(self, account_id, folder_id, name).await
    }

    async fn delete_folder_cap(
        &self,
        account_id: &str,
        folder_id: &str,
    ) -> Result<Vec<MailboxFolder>, BackendError> {
        folder_ops::delete_folder(self, account_id, folder_id).await
    }
}

#[async_trait]
impl MessageQueryProvider for ImapSmtpProvider {
    async fn list_messages_cap(
        &self,
        account_id: &str,
        folder_id: &str,
    ) -> Result<Vec<MailMessage>, BackendError> {
        memory::store().mail().list_messages(account_id, folder_id)
    }

    async fn search_messages_cap(
        &self,
        account_id: &str,
        query: &str,
    ) -> Result<Vec<MailMessage>, BackendError> {
        memory::store().mail().search_messages(account_id, query)
    }

    async fn get_message_cap(
        &self,
        account_id: &str,
        message_id: &str,
    ) -> Result<Option<MailMessage>, BackendError> {
        memory::store().mail().get_message(account_id, message_id)
    }

    async fn get_attachment_content_cap(
        &self,
        account_id: &str,
        message_id: &str,
        attachment_id: &str,
    ) -> Result<AttachmentContent, BackendError> {
        message_ops::get_attachment_content(account_id, message_id, attachment_id).await
    }
}

#[async_trait]
impl DraftProvider for ImapSmtpProvider {
    async fn get_draft_cap(
        &self,
        account_id: &str,
        draft_id: &str,
    ) -> Result<Option<DraftMessage>, BackendError> {
        message_ops::get_draft(account_id, draft_id).await
    }

    async fn save_draft_cap(&self, draft: DraftMessage) -> Result<DraftMessage, BackendError> {
        message_ops::save_draft(self, draft).await
    }

    async fn send_message_cap(&self, draft: DraftMessage) -> Result<String, BackendError> {
        message_ops::send_message(draft).await
    }
}

#[async_trait]
impl MessageMutationProvider for ImapSmtpProvider {
    async fn toggle_star_cap(
        &self,
        account_id: &str,
        message_id: &str,
    ) -> Result<Option<MailMessage>, BackendError> {
        message_ops::toggle_star(account_id, message_id).await
    }

    async fn toggle_read_cap(
        &self,
        account_id: &str,
        message_id: &str,
    ) -> Result<Option<MailMessage>, BackendError> {
        message_ops::toggle_read(account_id, message_id).await
    }

    async fn batch_toggle_read_cap(
        &self,
        account_id: &str,
        message_ids: &[String],
        is_read: bool,
    ) -> Result<(), BackendError> {
        message_ops::batch_toggle_read(account_id, message_ids, is_read).await
    }

    async fn delete_message_cap(&self, account_id: &str, message_id: &str) -> Result<(), BackendError> {
        message_ops::delete_message(account_id, message_id).await
    }

    async fn batch_delete_messages_cap(
        &self,
        account_id: &str,
        message_ids: &[String],
    ) -> Result<(), BackendError> {
        message_ops::batch_delete_messages(account_id, message_ids).await
    }

    async fn archive_message_cap(
        &self,
        account_id: &str,
        message_id: &str,
    ) -> Result<Option<MailMessage>, BackendError> {
        message_ops::archive_message(account_id, message_id).await
    }

    async fn restore_message_cap(
        &self,
        account_id: &str,
        message_id: &str,
    ) -> Result<Option<MailMessage>, BackendError> {
        message_ops::restore_message(account_id, message_id).await
    }

    async fn move_message_cap(
        &self,
        account_id: &str,
        message_id: &str,
        folder_id: &str,
    ) -> Result<Option<MailMessage>, BackendError> {
        message_ops::move_message(account_id, message_id, folder_id).await
    }

    async fn batch_move_messages_cap(
        &self,
        account_id: &str,
        message_ids: &[String],
        folder_id: &str,
    ) -> Result<(), BackendError> {
        message_ops::batch_move_messages(account_id, message_ids, folder_id).await
    }

    async fn mark_all_read_cap(&self, account_id: &str, folder_id: &str) -> Result<(), BackendError> {
        message_ops::mark_all_read(account_id, folder_id).await
    }
}

#[async_trait]
impl LabelProvider for ImapSmtpProvider {
    async fn list_labels_cap(&self, account_id: &str) -> Result<Vec<MailLabel>, BackendError> {
        label_ops::list_labels(account_id).await
    }

    async fn add_label_cap(
        &self,
        account_id: &str,
        message_id: &str,
        label: &str,
    ) -> Result<Option<MailMessage>, BackendError> {
        label_ops::add_label(account_id, message_id, label).await
    }

    async fn remove_label_cap(
        &self,
        account_id: &str,
        message_id: &str,
        label: &str,
    ) -> Result<Option<MailMessage>, BackendError> {
        label_ops::remove_label(account_id, message_id, label).await
    }

    async fn rename_label_cap(
        &self,
        account_id: &str,
        label: &str,
        new_label: &str,
    ) -> Result<Vec<MailLabel>, BackendError> {
        label_ops::rename_label(self, account_id, label, new_label).await
    }

    async fn delete_label_cap(
        &self,
        account_id: &str,
        label: &str,
    ) -> Result<Vec<MailLabel>, BackendError> {
        label_ops::delete_label(self, account_id, label).await
    }
}

#[async_trait]
impl SyncProvider for ImapSmtpProvider {
    async fn sync_account_cap(&self, account_id: &str) -> Result<SyncStatus, BackendError> {
        let account_state = memory::store().accounts().get_account_state(account_id)
            .ok_or_else(|| BackendError::not_found("Account not found"))?;
        sync_ops::sync_account_full(account_id, &account_state).await
    }

    async fn get_mailbox_bundle_cap(&self, account_id: &str) -> Result<MailboxBundle, BackendError> {
        memory::store().mail().get_mailbox_bundle(account_id)
    }

    async fn get_account_unread_snapshot_cap(
        &self,
        account_id: &str,
    ) -> Result<AccountUnreadSnapshot, BackendError> {
        memory::store().mail().get_account_unread_snapshot(account_id)
    }
}

// ---------------------------------------------------------------------------
// IMAP helpers (async)
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// IMAP helpers (async)
// ---------------------------------------------------------------------------

pub(crate) async fn wait_for_mailbox_change(
    state: &StoredAccountState,
    mailbox_name: &str,
    idle_timeout: Duration,
) -> Result<IdleMailboxChange, BackendError> {
    sync_ops::wait_for_mailbox_change(state, mailbox_name, idle_timeout).await
}

pub(crate) async fn poll_for_mailbox_change(
    state: &StoredAccountState,
    mailbox_name: &str,
    poll_interval: Duration,
) -> Result<IdleMailboxChange, BackendError> {
    sync_ops::poll_for_mailbox_change(state, mailbox_name, poll_interval).await
}

pub(crate) async fn sync_mailbox_incremental(
    account_id: &str,
    mailbox_name: &str,
) -> Result<SyncStatus, BackendError> {
    sync_ops::sync_mailbox_incremental(account_id, mailbox_name).await
}


// SMTP helper (async)
// ---------------------------------------------------------------------------

pub async fn smtp_send(
    state: &StoredAccountState,
    draft: &DraftMessage,
) -> Result<Vec<u8>, BackendError> {
    smtp_ops::smtp_send(state, draft).await
}
