use std::time::{Duration, Instant};

use async_trait::async_trait;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio_native_tls::TlsStream;

use crate::models::{
    AccountAuthMode, AccountSetupDraft, AttachmentContent, DraftMessage, MailAccount, MailFolderKind,
    MailMessage, MailThread, MailboxBundle, MailboxFolder, StoredAccountState, SyncStatus,
};
use crate::protocol::BackendError;
use crate::provider::common::{
    validate_draft, extract_body_from_mime, extract_attachments_from_mime,
    make_preview, find_mime_part_by_path, get_attachment_filename, base64_encode_bytes,
};
use crate::provider::imap::smtp_send;
use crate::provider::MailProvider;
use crate::storage::memory;
use crate::storage::persisted;

const TCP_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

pub struct Pop3SmtpProvider;

pub static POP3_SMTP_PROVIDER: Pop3SmtpProvider = Pop3SmtpProvider;

#[async_trait]
impl MailProvider for Pop3SmtpProvider {
    fn backend_name(&self) -> &'static str {
        "pop3-smtp"
    }

    async fn list_accounts(&self) -> Result<Vec<MailAccount>, BackendError> {
        memory::list_accounts()
    }

    async fn create_account(&self, draft: AccountSetupDraft) -> Result<MailAccount, BackendError> {
        eprintln!("[pop3] testing connection for new account {}...", draft.email);
        self.test_account_connection(draft.clone()).await?;
        eprintln!("[pop3] connection test passed, creating account");
        memory::create_account_without_test(draft)
    }

    async fn test_account_connection(&self, draft: AccountSetupDraft) -> Result<SyncStatus, BackendError> {
        validate_draft(&draft)?;
        eprintln!("[pop3] connecting to {}:{}...", draft.incoming_host, draft.incoming_port);
        let start = Instant::now();
        pop3_login_test(&draft).await?;
        eprintln!("[pop3] connection test ok ({:.1?})", start.elapsed());

        Ok(SyncStatus {
            account_id: "connection-test".into(),
            state: "idle".into(),
            message: format!(
                "Connected to {}:{} (POP3) and {}:{} (SMTP)",
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
        memory::toggle_star(account_id, message_id)
    }

    async fn toggle_read(&self, account_id: &str, message_id: &str) -> Result<Option<MailMessage>, BackendError> {
        memory::toggle_read(account_id, message_id)
    }

    async fn delete_message(&self, account_id: &str, message_id: &str) -> Result<(), BackendError> {
        memory::delete_message(account_id, message_id)
    }

    async fn delete_account(&self, account_id: &str) -> Result<(), BackendError> {
        eprintln!("[store] deleting account {account_id}");
        memory::delete_account(account_id)
    }

    async fn archive_message(&self, account_id: &str, message_id: &str) -> Result<Option<MailMessage>, BackendError> {
        memory::archive_message(account_id, message_id)
    }

    async fn restore_message(&self, account_id: &str, message_id: &str) -> Result<Option<MailMessage>, BackendError> {
        memory::restore_message(account_id, message_id)
    }

    async fn move_message(&self, account_id: &str, message_id: &str, folder_id: &str) -> Result<Option<MailMessage>, BackendError> {
        memory::move_message(account_id, message_id, folder_id)
    }

    async fn mark_all_read(&self, account_id: &str, folder_id: &str) -> Result<(), BackendError> {
        memory::mark_all_read(account_id, folder_id)
    }

    async fn sync_account(&self, account_id: &str) -> Result<SyncStatus, BackendError> {
        let account_state = memory::get_account_state(account_id)
            .ok_or_else(|| BackendError::not_found("Account not found"))?;

        eprintln!("[pop3] syncing account {} ({}:{})...", account_id, account_state.config.incoming_host, account_state.config.incoming_port);
        let start = Instant::now();
        let (folders, messages, threads) = pop3_fetch_mailbox(&account_state).await?;
        eprintln!("[pop3] fetched {} messages in {:.1?}", messages.len(), start.elapsed());
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
        let _ = account_id;
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
// POP3 login test
// ---------------------------------------------------------------------------

async fn pop3_login_test(draft: &AccountSetupDraft) -> Result<(), BackendError> {
    if matches!(draft.auth_mode, AccountAuthMode::Oauth) {
        return Err(BackendError::validation("POP3 does not support OAuth accounts in MailYou"));
    }

    let host = draft.incoming_host.trim();
    let port = draft.incoming_port;

    eprintln!("[pop3] connecting to {host}:{port} (tls={})...", draft.use_tls);

    let mut client = if draft.use_tls {
        Pop3Client::connect_tls(host, port).await?
    } else {
        Pop3Client::connect_plain(host, port).await?
    };

    eprintln!("[pop3] logging in as {}...", draft.username.trim());
    client.login(draft.username.trim(), draft.password.trim()).await?;
    client.quit().await?;

    Ok(())
}

// ---------------------------------------------------------------------------
// POP3 fetch mailbox
// ---------------------------------------------------------------------------

async fn pop3_fetch_mailbox(
    state: &StoredAccountState,
) -> Result<(Vec<MailboxFolder>, Vec<MailMessage>, Vec<MailThread>), BackendError> {
    if matches!(state.config.auth_mode, AccountAuthMode::Oauth) {
        return Err(BackendError::validation("POP3 does not support OAuth accounts in MailYou"));
    }

    let account_id = &state.account.id;
    let mut client = Pop3Client::connect_from_state(state).await?;

    client.login(&state.config.username, &state.config.password).await?;

    let stat = client.stat().await?;
    eprintln!("[pop3] mailbox has {} messages", stat.count);

    let uidl_map = client.uidl().await?;

    let existing_message_ids = memory::get_existing_message_ids(account_id);
    eprintln!("[pop3] incremental sync: {} cached messages available", existing_message_ids.len());

    let mut messages = Vec::new();
    let mut threads = Vec::new();

    for msg_num in 1..=stat.count {
        let uidl = uidl_map.get(&msg_num).map(|s| s.as_str()).unwrap_or("unknown");
        let message_id = format!("pop3-{account_id}-{uidl}");

        if existing_message_ids.contains(&message_id) {
            eprintln!("[pop3] message {msg_num} (uidl={uidl}) already cached, skipping");
            continue;
        }

        eprintln!("[pop3] fetching message {msg_num} (uidl={uidl})...");
        let raw = client.retr(msg_num).await?;
        let _ = persisted::save_raw_email(&message_id, &raw);

        let parsed = mailparse::parse_mail(&raw)
            .map_err(|e| BackendError::internal(format!("Failed to parse email: {e}")))?;

        let subject = parsed.headers.iter()
            .find(|h| h.get_key().eq_ignore_ascii_case("subject"))
            .map(|h| h.get_value())
            .unwrap_or_else(|| "(No subject)".into());

        let from = parsed.headers.iter()
            .find(|h| h.get_key().eq_ignore_ascii_case("from"))
            .map(|h| h.get_value())
            .unwrap_or_else(|| "Unknown".into());

        let date = parsed.headers.iter()
            .find(|h| h.get_key().eq_ignore_ascii_case("date"))
            .map(|h| h.get_value())
            .and_then(|d| mailparse::dateparse(&d).ok())
            .map(|ts| {
                chrono::DateTime::from_timestamp(ts, 0)
                    .unwrap_or_else(|| chrono::Utc::now())
                    .to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
            })
            .unwrap_or_else(|| memory::current_timestamp());

        let body = extract_body_from_mime(&raw);
        let preview = make_preview(&body);
        let attachments = extract_attachments_from_mime(&raw);

        let folder_id = format!("inbox-{account_id}");
        let thread_id = format!("thread-{message_id}");

        messages.push(MailMessage {
            id: message_id.clone(),
            account_id: account_id.clone(),
            folder_id: folder_id.clone(),
            thread_id: thread_id.clone(),
            subject: subject.clone(),
            preview,
            body,
            from: from.clone(),
            from_email: from.clone(),
            to: vec![],
            cc: vec![],
            sent_at: date.clone(),
            received_at: date.clone(),
            is_read: false,
            is_starred: false,
            has_attachments: !attachments.is_empty(),
            attachments,
            labels: vec![],
            imap_uid: None,
            previous_folder_id: None,
        });

        threads.push(MailThread {
            id: thread_id,
            account_id: account_id.clone(),
            subject,
            message_ids: vec![message_id],
            last_message_at: date,
            unread_count: 1,
        });
    }

    client.quit().await?;

    let inbox_folder = MailboxFolder {
        id: format!("inbox-{account_id}"),
        account_id: account_id.clone(),
        name: "Inbox".into(),
        kind: MailFolderKind::Inbox,
        unread_count: messages.iter().filter(|m| !m.is_read).count() as u32,
        total_count: messages.len() as u32,
        icon: "mdi-inbox".into(),
        imap_name: None,
    };

    let starred_folder = MailboxFolder {
        id: format!("starred-{account_id}"),
        account_id: account_id.clone(),
        name: "Starred".into(),
        kind: MailFolderKind::Starred,
        unread_count: 0,
        total_count: 0,
        icon: "mdi-star-outline".into(),
        imap_name: None,
    };

    Ok((vec![inbox_folder, starred_folder], messages, threads))
}

// ---------------------------------------------------------------------------
// Simple POP3 client
// ---------------------------------------------------------------------------

enum Pop3Stream {
    Plain(BufReader<TcpStream>),
    Tls(BufReader<TlsStream<TcpStream>>),
}

struct Pop3Client {
    stream: Pop3Stream,
}

#[derive(Debug)]
struct Pop3Stat {
    count: usize,
    _size: usize,
}

impl Pop3Client {
    async fn connect_plain(host: &str, port: u16) -> Result<Self, BackendError> {
        let tcp = tokio::time::timeout(TCP_CONNECT_TIMEOUT, TcpStream::connect((host, port)))
            .await
            .map_err(|_| BackendError::validation("POP3 connection timed out"))?
            .map_err(|e| BackendError::validation(format!("POP3 connection failed: {e}")))?;

        let mut client = Self {
            stream: Pop3Stream::Plain(BufReader::new(tcp)),
        };

        client.read_greeting().await?;
        Ok(client)
    }

    async fn connect_tls(host: &str, port: u16) -> Result<Self, BackendError> {
        let tcp = tokio::time::timeout(TCP_CONNECT_TIMEOUT, TcpStream::connect((host, port)))
            .await
            .map_err(|_| BackendError::validation("POP3 connection timed out"))?
            .map_err(|e| BackendError::validation(format!("POP3 connection failed: {e}")))?;

        let connector = native_tls::TlsConnector::new()
            .map_err(|e| BackendError::validation(format!("TLS error: {e}")))?;
        let connector = tokio_native_tls::TlsConnector::from(connector);
        let tls = connector.connect(host, tcp)
            .await
            .map_err(|e| BackendError::validation(format!("TLS handshake failed: {e}")))?;

        let mut client = Self {
            stream: Pop3Stream::Tls(BufReader::new(tls)),
        };

        client.read_greeting().await?;
        Ok(client)
    }

    async fn connect_from_state(state: &StoredAccountState) -> Result<Self, BackendError> {
        let host = state.config.incoming_host.trim();
        let port = state.config.incoming_port;

        if state.config.use_tls {
            Self::connect_tls(host, port).await
        } else {
            Self::connect_plain(host, port).await
        }
    }

    async fn read_greeting(&mut self) -> Result<(), BackendError> {
        let line = self.read_line().await?;
        if !line.starts_with("+OK") {
            return Err(BackendError::validation(format!("POP3 greeting failed: {line}")));
        }
        Ok(())
    }

    async fn read_line(&mut self) -> Result<String, BackendError> {
        let mut line = String::new();
        match &mut self.stream {
            Pop3Stream::Plain(reader) => {
                reader.read_line(&mut line).await
                    .map_err(|e| BackendError::internal(format!("POP3 read failed: {e}")))?;
            }
            Pop3Stream::Tls(reader) => {
                reader.read_line(&mut line).await
                    .map_err(|e| BackendError::internal(format!("POP3 read failed: {e}")))?;
            }
        }
        Ok(line.trim_end().to_string())
    }

    async fn write_line(&mut self, line: &str) -> Result<(), BackendError> {
        let data = format!("{line}\r\n");
        match &mut self.stream {
            Pop3Stream::Plain(reader) => {
                reader.get_mut().write_all(data.as_bytes()).await
                    .map_err(|e| BackendError::internal(format!("POP3 write failed: {e}")))?;
            }
            Pop3Stream::Tls(reader) => {
                reader.get_mut().write_all(data.as_bytes()).await
                    .map_err(|e| BackendError::internal(format!("POP3 write failed: {e}")))?;
            }
        }
        Ok(())
    }

    async fn command(&mut self, cmd: &str) -> Result<String, BackendError> {
        self.write_line(cmd).await?;
        let response = self.read_line().await?;
        if !response.starts_with("+OK") {
            return Err(BackendError::internal(format!("POP3 command failed: {response}")));
        }
        Ok(response)
    }

    async fn login(&mut self, username: &str, password: &str) -> Result<(), BackendError> {
        self.command(&format!("USER {username}")).await?;
        self.command(&format!("PASS {password}")).await?;
        Ok(())
    }

    async fn stat(&mut self) -> Result<Pop3Stat, BackendError> {
        let response = self.command("STAT").await?;
        let parts: Vec<&str> = response.split_whitespace().collect();
        if parts.len() < 3 {
            return Err(BackendError::internal("Invalid STAT response"));
        }
        let count = parts[1].parse().map_err(|_| BackendError::internal("Invalid STAT count"))?;
        let size = parts[2].parse().map_err(|_| BackendError::internal("Invalid STAT size"))?;
        Ok(Pop3Stat { count, _size: size })
    }

    async fn uidl(&mut self) -> Result<std::collections::HashMap<usize, String>, BackendError> {
        self.write_line("UIDL").await?;
        let response = self.read_line().await?;
        if !response.starts_with("+OK") {
            return Err(BackendError::internal(format!("UIDL failed: {response}")));
        }

        let mut map = std::collections::HashMap::new();
        loop {
            let line = self.read_line().await?;
            if line == "." {
                break;
            }
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                if let Ok(num) = parts[0].parse::<usize>() {
                    map.insert(num, parts[1].to_string());
                }
            }
        }
        Ok(map)
    }

    async fn retr(&mut self, msg_num: usize) -> Result<Vec<u8>, BackendError> {
        self.write_line(&format!("RETR {msg_num}")).await?;
        let response = self.read_line().await?;
        if !response.starts_with("+OK") {
            return Err(BackendError::internal(format!("RETR failed: {response}")));
        }

        let mut data = Vec::new();
        loop {
            let line = self.read_line().await?;
            if line == "." {
                break;
            }
            // POP3 byte-stuffing: lines starting with "." are escaped as ".."
            let line = if line.starts_with("..") {
                &line[1..]
            } else {
                &line
            };
            data.extend_from_slice(line.as_bytes());
            data.extend_from_slice(b"\r\n");
        }
        Ok(data)
    }

    async fn quit(&mut self) -> Result<(), BackendError> {
        self.command("QUIT").await?;
        Ok(())
    }
}
