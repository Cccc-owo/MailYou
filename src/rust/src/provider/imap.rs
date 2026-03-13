use std::net::{TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

use native_tls::TlsConnector;

use crate::models::{
    AccountSetupDraft, DraftMessage, MailAccount, MailFolderKind, MailMessage, MailThread,
    MailboxBundle, MailboxFolder, StoredAccountState, SyncStatus,
};
use crate::protocol::BackendError;
use crate::provider::MailProvider;
use crate::storage::memory;

const TCP_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const TCP_READ_TIMEOUT: Duration = Duration::from_secs(30);
const TCP_WRITE_TIMEOUT: Duration = Duration::from_secs(10);

pub struct ImapSmtpProvider;

pub static IMAP_SMTP_PROVIDER: ImapSmtpProvider = ImapSmtpProvider;

impl MailProvider for ImapSmtpProvider {
    fn backend_name(&self) -> &'static str {
        "imap-smtp"
    }

    fn list_accounts(&self) -> Result<Vec<MailAccount>, BackendError> {
        memory::list_accounts()
    }

    fn create_account(&self, draft: AccountSetupDraft) -> Result<MailAccount, BackendError> {
        eprintln!("[imap] testing connection for new account {}...", draft.email);
        self.test_account_connection(draft.clone())?;
        eprintln!("[imap] connection test passed, creating account");
        memory::create_account_without_test(draft)
    }

    fn test_account_connection(&self, draft: AccountSetupDraft) -> Result<SyncStatus, BackendError> {
        validate_draft(&draft)?;
        eprintln!("[imap] connecting to {}:{}...", draft.incoming_host, draft.incoming_port);
        let start = Instant::now();
        imap_login_test(&draft)?;
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

    fn list_folders(&self, account_id: &str) -> Result<Vec<MailboxFolder>, BackendError> {
        memory::list_folders(account_id)
    }

    fn list_messages(&self, account_id: &str, folder_id: &str) -> Result<Vec<MailMessage>, BackendError> {
        memory::list_messages(account_id, folder_id)
    }

    fn get_message(&self, account_id: &str, message_id: &str) -> Result<Option<MailMessage>, BackendError> {
        memory::get_message(account_id, message_id)
    }

    fn save_draft(&self, draft: DraftMessage) -> Result<DraftMessage, BackendError> {
        memory::save_draft(draft)
    }

    fn send_message(&self, draft: DraftMessage) -> Result<String, BackendError> {
        if draft.account_id.trim().is_empty() || draft.to.trim().is_empty() {
            return Err(BackendError::validation("Recipient and account are required"));
        }

        let account_state = memory::get_account_state(&draft.account_id)
            .ok_or_else(|| BackendError::not_found("Account not found"))?;

        eprintln!("[smtp] sending message to {} via {}:{}...", draft.to, account_state.config.outgoing_host, account_state.config.outgoing_port);
        let start = Instant::now();
        smtp_send(&account_state, &draft)?;
        eprintln!("[smtp] sent ok ({:.1?})", start.elapsed());
        memory::record_sent_message(draft)
    }

    fn toggle_star(&self, account_id: &str, message_id: &str) -> Result<Option<MailMessage>, BackendError> {
        let updated = memory::toggle_star(account_id, message_id)?;
        if let Some(ref msg) = updated {
            if let Some(uid) = msg.imap_uid {
                eprintln!("[imap] pushing star={} for uid {} in {}", msg.is_starred, uid, msg.folder_id);
                if let Err(e) = imap_store_flag(account_id, &msg.folder_id, uid, "\\Flagged", msg.is_starred) {
                    eprintln!("[imap] push star failed: {}", e.message);
                }
            }
        }
        Ok(updated)
    }

    fn toggle_read(&self, account_id: &str, message_id: &str) -> Result<Option<MailMessage>, BackendError> {
        let updated = memory::toggle_read(account_id, message_id)?;
        if let Some(ref msg) = updated {
            if let Some(uid) = msg.imap_uid {
                eprintln!("[imap] pushing read={} for uid {} in {}", msg.is_read, uid, msg.folder_id);
                if let Err(e) = imap_store_flag(account_id, &msg.folder_id, uid, "\\Seen", msg.is_read) {
                    eprintln!("[imap] push read failed: {}", e.message);
                }
            }
        }
        Ok(updated)
    }

    fn delete_message(&self, account_id: &str, message_id: &str) -> Result<(), BackendError> {
        let original = memory::get_message(account_id, message_id)?;
        memory::delete_message(account_id, message_id)?;

        if let Some(msg) = original {
            if let Some(uid) = msg.imap_uid {
                if let Ok(folders) = memory::list_folders(account_id) {
                    if let Some(trash) = folders.iter().find(|f| matches!(f.kind, MailFolderKind::Trash)) {
                        eprintln!("[imap] moving uid {} to trash", uid);
                        if let Err(e) = imap_move_message(account_id, &msg.folder_id, &trash.id, uid) {
                            eprintln!("[imap] push delete failed: {}", e.message);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn delete_account(&self, account_id: &str) -> Result<(), BackendError> {
        eprintln!("[store] deleting account {account_id}");
        memory::delete_account(account_id)
    }

    fn archive_message(&self, account_id: &str, message_id: &str) -> Result<Option<MailMessage>, BackendError> {
        let original = memory::get_message(account_id, message_id)?;
        let updated = memory::archive_message(account_id, message_id)?;

        if let (Some(orig), Some(ref upd)) = (original, &updated) {
            if let Some(uid) = orig.imap_uid {
                eprintln!("[imap] archiving uid {} from {} to {}", uid, orig.folder_id, upd.folder_id);
                if let Err(e) = imap_move_message(account_id, &orig.folder_id, &upd.folder_id, uid) {
                    eprintln!("[imap] push archive failed: {}", e.message);
                }
            }
        }
        Ok(updated)
    }

    fn restore_message(&self, account_id: &str, message_id: &str) -> Result<Option<MailMessage>, BackendError> {
        let original = memory::get_message(account_id, message_id)?;
        let updated = memory::restore_message(account_id, message_id)?;

        if let (Some(orig), Some(ref upd)) = (original, &updated) {
            if let Some(uid) = orig.imap_uid {
                eprintln!("[imap] restoring uid {} from {} to {}", uid, orig.folder_id, upd.folder_id);
                if let Err(e) = imap_move_message(account_id, &orig.folder_id, &upd.folder_id, uid) {
                    eprintln!("[imap] push restore failed: {}", e.message);
                }
            }
        }
        Ok(updated)
    }

    fn move_message(&self, account_id: &str, message_id: &str, folder_id: &str) -> Result<Option<MailMessage>, BackendError> {
        let original = memory::get_message(account_id, message_id)?;
        let updated = memory::move_message(account_id, message_id, folder_id)?;

        if let Some(orig) = original {
            if let Some(uid) = orig.imap_uid {
                eprintln!("[imap] moving uid {} from {} to {}", uid, orig.folder_id, folder_id);
                if let Err(e) = imap_move_message(account_id, &orig.folder_id, folder_id, uid) {
                    eprintln!("[imap] push move failed: {}", e.message);
                }
            }
        }
        Ok(updated)
    }

    fn mark_all_read(&self, account_id: &str, folder_id: &str) -> Result<(), BackendError> {
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
                    if let Ok(mut session) = imap_connect_by_account(account_id) {
                        if session.select(&mailbox_name).is_ok() {
                            for (uid, _) in &unread_uids {
                                let _ = session.uid_store(uid.to_string(), "+FLAGS (\\Seen)");
                            }
                        }
                        let _ = session.logout();
                    }
                }
            }
        }

        Ok(())
    }

    fn sync_account(&self, account_id: &str) -> Result<SyncStatus, BackendError> {
        let account_state = memory::get_account_state(account_id)
            .ok_or_else(|| BackendError::not_found("Account not found"))?;

        eprintln!("[imap] syncing account {} ({}:{})...", account_id, account_state.config.incoming_host, account_state.config.incoming_port);
        let start = Instant::now();
        let (folders, messages, threads) = imap_fetch_mailbox(&account_state)?;
        eprintln!("[imap] fetched {} folders, {} messages in {:.1?}", folders.len(), messages.len(), start.elapsed());
        memory::merge_remote_mailbox(account_id, folders, messages, threads)?;

        let timestamp = memory::current_timestamp();
        memory::finish_sync(account_id, &timestamp)
    }

    fn get_mailbox_bundle(&self, account_id: &str) -> Result<MailboxBundle, BackendError> {
        memory::get_mailbox_bundle(account_id)
    }
}

// ---------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------

fn validate_draft(draft: &AccountSetupDraft) -> Result<(), BackendError> {
    if draft.email.trim().is_empty()
        || draft.incoming_host.trim().is_empty()
        || draft.outgoing_host.trim().is_empty()
        || draft.username.trim().is_empty()
    {
        return Err(BackendError::validation("All account fields are required"));
    }

    if draft.incoming_port == 0 || draft.outgoing_port == 0 {
        return Err(BackendError::validation("Ports must be greater than 0"));
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// IMAP helpers
// ---------------------------------------------------------------------------

fn imap_login_test(draft: &AccountSetupDraft) -> Result<(), BackendError> {
    let host = draft.incoming_host.trim();
    let port = draft.incoming_port;

    let addr = (host, port)
        .to_socket_addrs()
        .map_err(|e| BackendError::validation(format!("DNS resolution failed: {e}")))?
        .next()
        .ok_or_else(|| BackendError::validation("Could not resolve IMAP server address"))?;

    eprintln!("[imap] tcp connecting to {addr}...");
    let tcp = TcpStream::connect_timeout(&addr, TCP_CONNECT_TIMEOUT)
        .map_err(|e| BackendError::validation(format!("IMAP connection failed: {e}")))?;
    tcp.set_read_timeout(Some(TCP_READ_TIMEOUT)).ok();
    tcp.set_write_timeout(Some(TCP_WRITE_TIMEOUT)).ok();
    eprintln!("[imap] tcp connected, tls={}...", draft.use_tls);

    if draft.use_tls {
        let tls = TlsConnector::builder()
            .build()
            .map_err(|e| BackendError::internal(format!("TLS error: {e}")))?;
        let tls_stream = tls
            .connect(host, tcp)
            .map_err(|e| BackendError::validation(format!("TLS handshake failed: {e}")))?;
        let client = imap::Client::new(tls_stream);
        eprintln!("[imap] logging in as {}...", draft.username.trim());
        let mut session = client
            .login(draft.username.trim(), draft.password.trim())
            .map_err(|e| BackendError::validation(format!("IMAP login failed: {}", e.0)))?;
        let _ = session.logout();
    } else {
        let client = imap::Client::new(tcp);
        eprintln!("[imap] logging in as {}...", draft.username.trim());
        let mut session = client
            .login(draft.username.trim(), draft.password.trim())
            .map_err(|e| BackendError::validation(format!("IMAP login failed: {}", e.0)))?;
        let _ = session.logout();
    }

    Ok(())
}

fn imap_connect(state: &StoredAccountState) -> Result<imap::Session<native_tls::TlsStream<std::net::TcpStream>>, BackendError> {
    let host = state.config.incoming_host.trim();
    let port = state.config.incoming_port;

    let addr = (host, port)
        .to_socket_addrs()
        .map_err(|e| BackendError::internal(format!("DNS resolution failed: {e}")))?
        .next()
        .ok_or_else(|| BackendError::internal("Could not resolve IMAP server address"))?;

    eprintln!("[imap] connecting to {host}:{port} ({addr})...");
    let start = Instant::now();
    let tcp = TcpStream::connect_timeout(&addr, TCP_CONNECT_TIMEOUT)
        .map_err(|e| BackendError::internal(format!("IMAP connection failed: {e}")))?;
    tcp.set_read_timeout(Some(TCP_READ_TIMEOUT)).ok();
    tcp.set_write_timeout(Some(TCP_WRITE_TIMEOUT)).ok();

    let tls = TlsConnector::builder()
        .build()
        .map_err(|e| BackendError::internal(format!("TLS error: {e}")))?;

    let tls_stream = tls
        .connect(host, tcp)
        .map_err(|e| BackendError::internal(format!("TLS handshake failed: {e}")))?;

    let client = imap::Client::new(tls_stream);
    let session = client
        .login(state.config.username.trim(), state.config.password.trim())
        .map_err(|e| BackendError::internal(format!("IMAP login failed: {}", e.0)))?;

    eprintln!("[imap] connected to {host}:{port} ({:.1?})", start.elapsed());
    Ok(session)
}

fn imap_connect_by_account(account_id: &str) -> Result<imap::Session<native_tls::TlsStream<std::net::TcpStream>>, BackendError> {
    let state = memory::get_account_state(account_id)
        .ok_or_else(|| BackendError::not_found("Account not found"))?;
    imap_connect(&state)
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

fn imap_store_flag(
    account_id: &str,
    folder_id: &str,
    uid: u32,
    flag: &str,
    add: bool,
) -> Result<(), BackendError> {
    let mailbox_name = get_imap_folder_name(account_id, folder_id)
        .ok_or_else(|| BackendError::internal("IMAP folder name not found"))?;

    let mut session = imap_connect_by_account(account_id)?;
    session
        .select(&mailbox_name)
        .map_err(|e| BackendError::internal(format!("IMAP SELECT failed: {e}")))?;

    let query = if add {
        format!("+FLAGS ({})", flag)
    } else {
        format!("-FLAGS ({})", flag)
    };

    session
        .uid_store(uid.to_string(), &query)
        .map_err(|e| BackendError::internal(format!("IMAP STORE failed: {e}")))?;

    let _ = session.logout();
    Ok(())
}

fn imap_move_message(
    account_id: &str,
    src_folder_id: &str,
    dest_folder_id: &str,
    uid: u32,
) -> Result<(), BackendError> {
    let src_name = get_imap_folder_name(account_id, src_folder_id)
        .ok_or_else(|| BackendError::internal("Source IMAP folder name not found"))?;
    let dest_name = get_imap_folder_name(account_id, dest_folder_id)
        .ok_or_else(|| BackendError::internal("Destination IMAP folder name not found"))?;

    let mut session = imap_connect_by_account(account_id)?;
    session
        .select(&src_name)
        .map_err(|e| BackendError::internal(format!("IMAP SELECT failed: {e}")))?;

    let uid_str = uid.to_string();
    session
        .uid_copy(&uid_str, &dest_name)
        .map_err(|e| BackendError::internal(format!("IMAP COPY failed: {e}")))?;

    session
        .uid_store(&uid_str, "+FLAGS (\\Deleted)")
        .map_err(|e| BackendError::internal(format!("IMAP STORE \\Deleted failed: {e}")))?;

    session
        .expunge()
        .map_err(|e| BackendError::internal(format!("IMAP EXPUNGE failed: {e}")))?;

    let _ = session.logout();
    Ok(())
}

fn imap_fetch_mailbox(
    state: &StoredAccountState,
) -> Result<(Vec<MailboxFolder>, Vec<MailMessage>, Vec<MailThread>), BackendError> {
    let mut session = imap_connect(state)?;
    let account_id = &state.account.id;

    // Collect existing message bodies from memory so we can skip re-downloading them.
    let existing_bodies = memory::get_existing_bodies(account_id);
    eprintln!("[imap] incremental sync: {} cached bodies available", existing_bodies.len());

    let remote_folders = session
        .list(None, Some("*"))
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

    for remote_folder in remote_folders.iter() {
        let raw_name = remote_folder.name();

        if remote_folder.attributes().iter().any(|a| matches!(a, imap::types::NameAttribute::NoSelect)) {
            continue;
        }

        let display_name = decode_imap_utf7(raw_name);
        let (kind, icon) = classify_folder(&display_name);
        let folder_id = format!("{}-{}", slug(raw_name), account_id);

        let (unread, total, messages, threads) =
            fetch_folder_contents(&mut session, account_id, &folder_id, raw_name, &existing_bodies)?;

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

    let _ = session.logout();
    Ok((folders, all_messages, all_threads))
}

/// Cached body + preview for a message, keyed by IMAP UID.
type ExistingBodies = std::collections::HashMap<u32, (String, String)>;

fn fetch_folder_contents(
    session: &mut imap::Session<native_tls::TlsStream<std::net::TcpStream>>,
    account_id: &str,
    folder_id: &str,
    mailbox_name: &str,
    existing_bodies: &ExistingBodies,
) -> Result<(u32, u32, Vec<MailMessage>, Vec<MailThread>), BackendError> {
    let mailbox = session
        .select(mailbox_name)
        .map_err(|e| BackendError::internal(format!("IMAP SELECT '{mailbox_name}' failed: {e}")))?;

    let total = mailbox.exists;

    if total == 0 {
        return Ok((0, total, Vec::new(), Vec::new()));
    }

    // SEARCH UNSEEN to get the actual unread count (SELECT's UNSEEN is just
    // the sequence number of the first unseen message, not the count).
    let unread = session
        .search("UNSEEN")
        .map(|ids| ids.len() as u32)
        .unwrap_or(0);

    // Fetch the most recent 50 messages (or all if fewer)
    let start = if total > 50 { total - 49 } else { 1 };
    let range = format!("{start}:{total}");

    // --- Phase 1: lightweight fetch (envelope + flags, no body) ---
    let fetches = session
        .fetch(&range, "(UID FLAGS ENVELOPE RFC822.SIZE)")
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

    for fetch in fetches.iter() {
        let uid = fetch.uid.unwrap_or(fetch.message);
        let message_id = format!("imap-{account_id}-{folder_id}-{uid}");
        let thread_id = format!("thread-{message_id}");

        let flags = fetch.flags();
        let is_read = flags.iter().any(|f| matches!(f, imap::types::Flag::Seen));
        let is_starred = flags.iter().any(|f| matches!(f, imap::types::Flag::Flagged));

        let (subject, from, from_email, to, cc, date) = match fetch.envelope() {
            Some(env) => parse_envelope(env),
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
        if let Ok(body_fetches) = session.uid_fetch(&uid_set, "BODY.PEEK[]") {
            for bf in body_fetches.iter() {
                if let (Some(uid), Some(raw)) = (bf.uid, bf.body()) {
                    body_map.insert(uid, raw.to_vec());
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
        let (body, preview) = if let Some((cached_body, cached_preview)) = existing_bodies.get(&meta.uid) {
            // Reuse body + preview from memory
            (cached_body.clone(), cached_preview.clone())
        } else if let Some(raw) = body_map.remove(&meta.uid) {
            // Parse newly downloaded body
            let parsed = extract_body_from_mime(&raw);
            let prev = make_preview(&parsed);
            (parsed, prev)
        } else {
            (String::new(), "(No preview)".into())
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
            sent_at: meta.date.clone(),
            received_at: meta.date,
            is_read: meta.is_read,
            is_starred: meta.is_starred,
            has_attachments: false,
            attachments: vec![],
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

fn make_preview(body: &str) -> String {
    let preview = strip_html_tags(body)
        .chars()
        .take(96)
        .collect::<String>()
        .replace('\n', " ")
        .replace('\r', "");
    if preview.is_empty() {
        "(No preview)".into()
    } else {
        preview
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
        .map(|d| String::from_utf8_lossy(d).to_string())
        .unwrap_or_else(|| memory::current_timestamp());

    (subject, from, from_email, to, cc, date)
}

fn decode_header_value(raw: &[u8]) -> String {
    let lossy = String::from_utf8_lossy(raw).to_string();
    decode_rfc2047(&lossy)
}

/// Decode RFC 2047 encoded-words: =?charset?encoding?text?=
fn decode_rfc2047(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut pos = 0;
    let bytes = input.as_bytes();

    while pos < bytes.len() {
        // Find next =?
        match input[pos..].find("=?") {
            None => {
                result.push_str(&input[pos..]);
                break;
            }
            Some(offset) => {
                result.push_str(&input[pos..pos + offset]);
                pos += offset;

                match decode_one_encoded_word(&input[pos..]) {
                    Some((decoded, consumed)) => {
                        result.push_str(&decoded);
                        pos += consumed;

                        // Skip whitespace between consecutive encoded-words
                        let after = &input[pos..];
                        let trimmed = after.trim_start_matches(|c: char| c == ' ' || c == '\t' || c == '\r' || c == '\n');
                        if trimmed.starts_with("=?") {
                            pos = input.len() - trimmed.len();
                        }
                    }
                    None => {
                        result.push_str("=?");
                        pos += 2;
                    }
                }
            }
        }
    }

    result
}

/// Try to decode one encoded-word at the start of `input`.
/// Returns (decoded_string, bytes_consumed) or None.
fn decode_one_encoded_word(input: &str) -> Option<(String, usize)> {
    if !input.starts_with("=?") {
        return None;
    }

    let rest = &input[2..];
    let q1 = rest.find('?')?;
    let charset = &rest[..q1];

    let rest = &rest[q1 + 1..];
    let q2 = rest.find('?')?;
    let encoding = &rest[..q2];

    let rest = &rest[q2 + 1..];
    let end = rest.find("?=")?;
    let encoded_text = &rest[..end];

    let consumed = 2 + q1 + 1 + q2 + 1 + end + 2;

    let decoded_bytes = match encoding.to_uppercase().as_str() {
        "B" => base64_decode(encoded_text)?,
        "Q" => qp_decode_rfc2047(encoded_text),
        _ => return None,
    };

    let decoded_str = charset_decode(&decoded_bytes, charset);
    Some((decoded_str, consumed))
}

fn base64_decode(input: &str) -> Option<Vec<u8>> {
    let input = input.replace(['\r', '\n', ' '], "");
    let table: Vec<u8> = (0..256)
        .map(|i| match i as u8 as char {
            'A'..='Z' => (i - b'A' as usize) as u8,
            'a'..='z' => (i - b'a' as usize + 26) as u8,
            '0'..='9' => (i - b'0' as usize + 52) as u8,
            '+' => 62,
            '/' => 63,
            _ => 0xFF,
        })
        .collect();

    let mut out = Vec::with_capacity(input.len() * 3 / 4);
    let bytes: Vec<u8> = input.bytes().filter(|&b| b != b'=').collect();

    for chunk in bytes.chunks(4) {
        let vals: Vec<u8> = chunk.iter().map(|&b| table[b as usize]).collect();
        if vals.iter().any(|&v| v == 0xFF) {
            return None;
        }
        if vals.len() >= 2 {
            out.push((vals[0] << 2) | (vals[1] >> 4));
        }
        if vals.len() >= 3 {
            out.push((vals[1] << 4) | (vals[2] >> 2));
        }
        if vals.len() >= 4 {
            out.push((vals[2] << 6) | vals[3]);
        }
    }

    Some(out)
}

fn qp_decode_rfc2047(input: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'_' {
            out.push(b' ');
            i += 1;
        } else if bytes[i] == b'=' && i + 2 < bytes.len() {
            if let Ok(byte) = u8::from_str_radix(
                &String::from_utf8_lossy(&bytes[i + 1..i + 3]),
                16,
            ) {
                out.push(byte);
                i += 3;
            } else {
                out.push(bytes[i]);
                i += 1;
            }
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    out
}

fn charset_decode(bytes: &[u8], charset: &str) -> String {
    match charset.to_lowercase().as_str() {
        "utf-8" | "utf8" => String::from_utf8_lossy(bytes).to_string(),
        "iso-8859-1" | "latin1" | "latin-1" | "us-ascii" | "ascii" => {
            bytes.iter().map(|&b| b as char).collect()
        }
        _ => {
            // For other charsets (gbk, gb2312, big5, iso-2022-jp, etc.)
            // try encoding_rs via mailparse's internal charset handling
            // Fallback: best-effort UTF-8 lossy
            String::from_utf8_lossy(bytes).to_string()
        }
    }
}

// ---------------------------------------------------------------------------
// SMTP helper
// ---------------------------------------------------------------------------

fn smtp_send(state: &StoredAccountState, draft: &DraftMessage) -> Result<(), BackendError> {
    use lettre::message::Mailbox;
    use lettre::transport::smtp::authentication::Credentials;
    use lettre::{Message, SmtpTransport, Transport};

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

    let email = builder
        .subject(&draft.subject)
        .body(draft.body.clone())
        .map_err(|e| BackendError::internal(format!("Failed to build email: {e}")))?;

    let host = state.config.outgoing_host.trim();
    let port = state.config.outgoing_port;
    let creds = Credentials::new(
        state.config.username.clone(),
        state.config.password.clone(),
    );

    let transport = if state.config.use_tls {
        SmtpTransport::relay(host)
            .map_err(|e| BackendError::internal(format!("SMTP relay error: {e}")))?
            .port(port)
            .credentials(creds)
            .build()
    } else {
        SmtpTransport::builder_dangerous(host)
            .port(port)
            .credentials(creds)
            .build()
    };

    transport
        .send(&email)
        .map_err(|e| BackendError::internal(format!("SMTP send failed: {e}")))?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Utilities
// ---------------------------------------------------------------------------

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

fn extract_body_from_mime(raw: &[u8]) -> String {
    let parsed = match mailparse::parse_mail(raw) {
        Ok(mail) => mail,
        Err(_) => return String::from_utf8_lossy(raw).to_string(),
    };

    // Try to find text/html first, then text/plain
    let mut body = if let Some(html) = find_mime_part(&parsed, "text/html") {
        html
    } else if let Some(plain) = find_mime_part(&parsed, "text/plain") {
        format!("<pre style=\"white-space: pre-wrap; word-wrap: break-word;\">{}</pre>", html_escape(&plain))
    } else {
        parsed.get_body().unwrap_or_default()
    };

    // Resolve cid: inline image references to data: URIs
    let cid_map = collect_cid_parts(&parsed);
    for (cid, data_uri) in &cid_map {
        body = body.replace(&format!("cid:{cid}"), data_uri);
    }

    body
}

fn find_mime_part(mail: &mailparse::ParsedMail, target_type: &str) -> Option<String> {
    let content_type = mail
        .ctype
        .mimetype
        .to_lowercase();

    if content_type == target_type {
        return mail.get_body().ok();
    }

    for subpart in &mail.subparts {
        if let Some(body) = find_mime_part(subpart, target_type) {
            return Some(body);
        }
    }

    None
}

/// Collect all MIME parts that have a Content-ID header and image/* content type,
/// returning (content_id, data_uri) pairs for cid: replacement.
fn collect_cid_parts(mail: &mailparse::ParsedMail) -> Vec<(String, String)> {
    let mut result = Vec::new();
    collect_cid_parts_recursive(mail, &mut result);
    result
}

fn collect_cid_parts_recursive(mail: &mailparse::ParsedMail, result: &mut Vec<(String, String)>) {
    let content_id = mail.headers.iter()
        .find(|h| h.get_key().eq_ignore_ascii_case("content-id"))
        .map(|h| {
            let val = h.get_value();
            let trimmed = val.trim();
            // Strip angle brackets: <content-id> -> content-id
            if trimmed.starts_with('<') && trimmed.ends_with('>') {
                trimmed[1..trimmed.len() - 1].to_string()
            } else {
                trimmed.to_string()
            }
        });

    if let Some(cid) = content_id {
        let mime_type = &mail.ctype.mimetype;
        if mime_type.starts_with("image/") {
            if let Ok(raw_body) = mail.get_body_raw() {
                let b64 = base64_encode_bytes(&raw_body);
                result.push((cid, format!("data:{mime_type};base64,{b64}")));
            }
        }
    }

    for subpart in &mail.subparts {
        collect_cid_parts_recursive(subpart, result);
    }
}

fn base64_encode_bytes(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity((input.len() + 2) / 3 * 4);

    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;

        result.push(TABLE[((triple >> 18) & 0x3F) as usize] as char);
        result.push(TABLE[((triple >> 12) & 0x3F) as usize] as char);

        if chunk.len() > 1 {
            result.push(TABLE[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }

        if chunk.len() > 2 {
            result.push(TABLE[(triple & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }

    result
}

fn strip_html_tags(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut in_tag = false;
    for ch in html.chars() {
        if ch == '<' {
            in_tag = true;
        } else if ch == '>' {
            in_tag = false;
        } else if !in_tag {
            result.push(ch);
        }
    }
    result
}

fn html_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Decode IMAP Modified UTF-7 folder names (RFC 3501 §5.1.3).
///
/// Rules:
///   - ASCII printable chars (0x20–0x7E) except '&' pass through
///   - `&-` decodes to literal `&`
///   - `&<modified-base64>-` decodes to UTF-16BE via base64 (with `,` in place of `/`)
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

        // Found '&'
        i += 1;

        // &- means literal '&'
        if i < bytes.len() && bytes[i] == b'-' {
            result.push('&');
            i += 1;
            continue;
        }

        // Collect modified base64 until '-'
        let start = i;
        while i < bytes.len() && bytes[i] != b'-' {
            i += 1;
        }

        let encoded = &input[start..i];

        // Skip the closing '-'
        if i < bytes.len() {
            i += 1;
        }

        // Decode: replace ',' with '/' to get standard base64, then decode to UTF-16BE
        let standard_b64: String = encoded.chars().map(|c| if c == ',' { '/' } else { c }).collect();

        if let Some(utf16_bytes) = base64_decode(&standard_b64) {
            let utf16: Vec<u16> = utf16_bytes
                .chunks(2)
                .filter(|c| c.len() == 2)
                .map(|c| u16::from_be_bytes([c[0], c[1]]))
                .collect();

            match String::from_utf16(&utf16) {
                Ok(decoded) => result.push_str(&decoded),
                Err(_) => {
                    // Fallback: keep raw
                    result.push('&');
                    result.push_str(encoded);
                    result.push('-');
                }
            }
        } else {
            result.push('&');
            result.push_str(encoded);
            result.push('-');
        }
    }

    result
}
