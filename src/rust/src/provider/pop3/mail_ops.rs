use std::time::Instant;

use crate::models::{
    AccountAuthMode, AccountSetupDraft, AttachmentContent, DraftMessage, MailMessage, MailThread,
    MailboxFolder, StoredAccountState, SyncStatus, MailFolderKind,
};
use crate::protocol::BackendError;
use crate::provider::common::{
    extract_attachments_from_mime, extract_body_from_mime, finalize_smtp_send,
    get_attachment_content_from_storage, log_smtp_send_start, make_preview, prepare_smtp_send,
    validate_draft,
};
use crate::storage::{memory, persisted};

use super::client::Pop3Client;

pub(super) async fn test_account_connection(
    draft: AccountSetupDraft,
) -> Result<SyncStatus, BackendError> {
    validate_draft(&draft)?;
    eprintln!(
        "[pop3] connecting to {}:{}...",
        draft.incoming_host, draft.incoming_port
    );
    let start = Instant::now();
    Pop3Client::login_test(&draft).await?;
    eprintln!("[pop3] connection test ok ({:.1?})", start.elapsed());

    Ok(crate::provider::common::build_connection_test_status(format!(
            "Connected to {}:{} (POP3) and {}:{} (SMTP)",
            draft.incoming_host, draft.incoming_port, draft.outgoing_host, draft.outgoing_port
        )))
}

pub(super) async fn send_message(draft: DraftMessage) -> Result<String, BackendError> {
    let account_state = prepare_smtp_send(&draft)?;
    let start = log_smtp_send_start(&draft, &account_state);
    let raw_email = crate::provider::imap::smtp_send(&account_state, &draft).await?;
    finalize_smtp_send(draft, raw_email, start)
}

pub(super) async fn sync_account(account_id: &str) -> Result<SyncStatus, BackendError> {
    let accounts = memory::store().accounts();
    let mail = memory::store().mail();
    let account_state = accounts.get_account_state(account_id)
        .ok_or_else(|| BackendError::not_found("Account not found"))?;

    eprintln!(
        "[pop3] syncing account {} ({}:{})...",
        account_id, account_state.config.incoming_host, account_state.config.incoming_port
    );
    let start = Instant::now();
    let (folders, messages, threads) = pop3_fetch_mailbox(&account_state).await?;
    eprintln!(
        "[pop3] fetched {} messages in {:.1?}",
        messages.len(),
        start.elapsed()
    );
    mail.merge_remote_mailbox(account_id, folders, messages, threads)?;

    let timestamp = memory::current_timestamp();
    accounts.finish_sync(account_id, &timestamp)
}

pub(super) async fn get_attachment_content(
    account_id: &str,
    message_id: &str,
    attachment_id: &str,
) -> Result<AttachmentContent, BackendError> {
    get_attachment_content_from_storage(account_id, message_id, attachment_id)
}

async fn pop3_fetch_mailbox(
    state: &StoredAccountState,
) -> Result<(Vec<MailboxFolder>, Vec<MailMessage>, Vec<MailThread>), BackendError> {
    if matches!(state.config.auth_mode, AccountAuthMode::Oauth) {
        return Err(BackendError::validation(
            "POP3 does not support OAuth accounts in MailYou",
        ));
    }

    let account_id = &state.account.id;
    let mut client = Pop3Client::connect_from_state(state).await?;

    client
        .login(&state.config.username, &state.config.password)
        .await?;

    let stat = client.stat().await?;
    eprintln!("[pop3] mailbox has {} messages", stat.count);

    let uidl_map = client.uidl().await?;

    let existing_message_ids = memory::store().mail().get_existing_message_ids(account_id);
    eprintln!(
        "[pop3] incremental sync: {} cached messages available",
        existing_message_ids.len()
    );

    let mut messages = Vec::new();
    let mut threads = Vec::new();

    for msg_num in 1..=stat.count {
        let uidl = uidl_map
            .get(&msg_num)
            .map(|s| s.as_str())
            .unwrap_or("unknown");
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

        let subject = parsed
            .headers
            .iter()
            .find(|h| h.get_key().eq_ignore_ascii_case("subject"))
            .map(|h| h.get_value())
            .unwrap_or_else(|| "(No subject)".into());

        let from = parsed
            .headers
            .iter()
            .find(|h| h.get_key().eq_ignore_ascii_case("from"))
            .map(|h| h.get_value())
            .unwrap_or_else(|| "Unknown".into());

        let date = parsed
            .headers
            .iter()
            .find(|h| h.get_key().eq_ignore_ascii_case("date"))
            .map(|h| h.get_value())
            .and_then(|d| mailparse::dateparse(&d).ok())
            .map(|ts| {
                chrono::DateTime::from_timestamp(ts, 0)
                    .unwrap_or_else(chrono::Utc::now)
                    .to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
            })
            .unwrap_or_else(memory::current_timestamp);

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
        imap_uid_validity: None,
        imap_uid_next: None,
        imap_highest_modseq: None,
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
        imap_uid_validity: None,
        imap_uid_next: None,
        imap_highest_modseq: None,
    };

    Ok((vec![inbox_folder, starred_folder], messages, threads))
}
