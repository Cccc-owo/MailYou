use futures::TryStreamExt;

use crate::models::{
    MailFolderKind, MailMessage, MailThread, MailboxFolder, StoredAccountState, SyncStatus,
};
use crate::protocol::BackendError;
use crate::storage::{memory, persisted};

pub(super) async fn wait_for_mailbox_change(
    state: &StoredAccountState,
    mailbox_name: &str,
    idle_timeout: std::time::Duration,
) -> Result<super::IdleMailboxChange, BackendError> {
    let mut session = super::client_ops::imap_connect(state).await?;
    let account_id = &state.account.id;
    let mail = memory::store().mail();
    let folder = mail.list_folders(account_id).ok().and_then(|folders| {
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
        async_imap::extensions::idle::IdleResponse::Timeout => {
            Ok(super::IdleMailboxChange::Timeout)
        }
        async_imap::extensions::idle::IdleResponse::ManualInterrupt => {
            Ok(super::IdleMailboxChange::Changed)
        }
        async_imap::extensions::idle::IdleResponse::NewData(data) => match data.parsed() {
            imap_proto::Response::Vanished { uids, .. } => {
                let flattened = uids
                    .iter()
                    .flat_map(|range| range.clone())
                    .collect::<Vec<_>>();
                Ok(super::IdleMailboxChange::Vanished(flattened))
            }
            _ => Ok(super::IdleMailboxChange::Changed),
        },
    }
}

pub(super) async fn sync_mailbox_incremental(
    account_id: &str,
    mailbox_name: &str,
) -> Result<SyncStatus, BackendError> {
    let accounts = memory::store().accounts();
    let mail = memory::store().mail();
    let account_state = accounts
        .get_account_state(account_id)
        .ok_or_else(|| BackendError::not_found("Account not found"))?;
    let folder = resolve_or_create_folder(account_id, mailbox_name);
    let existing_bodies = mail.get_existing_bodies_for_folder(account_id, &folder.id);
    let known_max_uid = mail.get_max_imap_uid_for_folder(account_id, &folder.id);

    let mut session = super::client_ops::imap_connect(&account_state).await?;
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
        mail.remove_messages_by_imap_uids(account_id, &folder.id, &sync_result.vanished_uids)?;
    }

    let synced_folder = MailboxFolder {
        unread_count: sync_result.unread,
        total_count: sync_result.total,
        imap_uid_validity: sync_result.uid_validity,
        imap_uid_next: sync_result.uid_next,
        imap_highest_modseq: sync_result.highest_modseq,
        ..folder
    };

    mail.merge_remote_folder(
        account_id,
        synced_folder,
        sync_result.messages,
        sync_result.threads,
        sync_result.fetched_all_messages,
    )?;

    let timestamp = memory::current_timestamp();
    accounts.finish_sync(account_id, &timestamp)
}

pub(super) async fn sync_account_full(
    account_id: &str,
    account_state: &StoredAccountState,
) -> Result<SyncStatus, BackendError> {
    let mail = memory::store().mail();
    let accounts = memory::store().accounts();
    eprintln!(
        "[imap] syncing account {} ({}:{})...",
        account_id, account_state.config.incoming_host, account_state.config.incoming_port
    );
    let start = std::time::Instant::now();
    let (folders, messages, threads) = imap_fetch_mailbox(account_state).await?;
    eprintln!(
        "[imap] fetched {} folders, {} messages in {:.1?}",
        folders.len(),
        messages.len(),
        start.elapsed()
    );
    mail.merge_remote_mailbox(account_id, folders, messages, threads)?;

    let timestamp = memory::current_timestamp();
    accounts.finish_sync(account_id, &timestamp)
}

async fn imap_fetch_mailbox(
    state: &StoredAccountState,
) -> Result<(Vec<MailboxFolder>, Vec<MailMessage>, Vec<MailThread>), BackendError> {
    let account_id = state.account.id.clone();
    let mut session = super::client_ops::imap_connect(state).await?;
    let remote_folders = session
        .list(None, Some("*"))
        .await
        .map_err(|e| BackendError::internal(format!("IMAP LIST failed: {e}")))?
        .try_collect::<Vec<_>>()
        .await
        .map_err(|e| BackendError::internal(format!("IMAP LIST failed: {e}")))?;

    let existing_bodies = memory::store().mail().get_existing_bodies(&account_id);
    let mut folders = Vec::new();
    let mut all_messages = Vec::new();
    let mut all_threads = Vec::new();

    folders.push(MailboxFolder {
        id: format!("starred-{}", account_id),
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

        let display_name = super::parse_ops::decode_imap_utf7(raw_name);
        let (kind, icon) = classify_folder(&display_name);
        let folder_id = format!("{}-{}", super::parse_ops::slug(raw_name), account_id);

        let (unread, total, messages, threads) = fetch_folder_contents(
            &mut session,
            &account_id,
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
    session: &mut super::ImapAnySession,
    account_id: &str,
    folder_id: &str,
    mailbox_name: &str,
    existing_bodies: &super::ExistingBodies,
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
    session: &mut super::ImapAnySession,
    account_id: &str,
    folder_id: &str,
    mailbox_name: &str,
    existing_bodies: &super::ExistingBodies,
    known_max_uid: Option<u32>,
    recent_limit: u32,
) -> Result<super::FolderSyncResult, BackendError> {
    let previous_folder = memory::store()
        .mail()
        .list_folders(account_id)
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
    let uid_validity_changed =
        previous_uid_validity.is_some() && uid_validity.is_some() && previous_uid_validity != uid_validity;
    let use_changedsince =
        !uid_validity_changed && previous_highest_modseq.is_some() && highest_modseq.is_some();

    if total == 0 {
        return Ok(super::FolderSyncResult {
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
    } else if has_gmail_labels {
        "(UID FLAGS ENVELOPE RFC822.SIZE MODSEQ X-GM-LABELS)".into()
    } else {
        "(UID FLAGS ENVELOPE RFC822.SIZE MODSEQ)".into()
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

    let mut metas: Vec<super::MsgMeta> = Vec::with_capacity(fetches.len());
    let mut new_uids: Vec<u32> = Vec::new();
    let mut labels_by_uid = std::collections::HashMap::new();

    if has_gmail_labels && !fetches.is_empty() {
        let uid_set = fetches
            .iter()
            .map(|fetch| fetch.uid.unwrap_or(fetch.message).to_string())
            .collect::<Vec<_>>()
            .join(",");
        labels_by_uid = super::label_ops::imap_fetch_gmail_labels(session, &uid_set).await?;
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
                        super::label_ops::normalize_keyword_label(value.as_ref())
                    }
                    _ => None,
                })
                .collect()
        };

        let (subject, from, from_email, to, cc, date) = match fetch.envelope() {
            Some(env) => {
                let (subj, frm, frm_email, t, c, d) = super::parse_ops::parse_envelope(env);
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

        metas.push(super::MsgMeta {
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

    let mut messages = Vec::with_capacity(metas.len());
    let mut threads = Vec::with_capacity(metas.len());

    for meta in metas {
        let (body, preview, attachments, final_date) = if meta.date.is_empty() {
            if let Some((cached_body, cached_preview, cached_attachments, cached_date)) =
                existing_bodies.get(&meta.uid)
            {
                if !cached_date.is_empty() && !cached_date.starts_with("2026-03-16") {
                    (
                        cached_body.clone(),
                        cached_preview.clone(),
                        cached_attachments.clone(),
                        cached_date.clone(),
                    )
                } else if let Some(raw) = body_map.remove(&meta.uid) {
                    let parsed = super::extract_body_from_mime(&raw);
                    let prev = super::make_preview(&parsed);
                    let atts = super::extract_attachments_from_mime(&raw);
                    let date = super::parse_ops::extract_date_from_body(&raw, meta.uid);
                    (parsed, prev, atts, date)
                } else {
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
                let parsed = super::extract_body_from_mime(&raw);
                let prev = super::make_preview(&parsed);
                let atts = super::extract_attachments_from_mime(&raw);
                let date = super::parse_ops::extract_date_from_body(&raw, meta.uid);
                (parsed, prev, atts, date)
            } else {
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
            (
                cached_body.clone(),
                cached_preview.clone(),
                cached_attachments.clone(),
                meta.date.clone(),
            )
        } else if let Some(raw) = body_map.remove(&meta.uid) {
            let parsed = super::extract_body_from_mime(&raw);
            let prev = super::make_preview(&parsed);
            let atts = super::extract_attachments_from_mime(&raw);
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

    Ok(super::FolderSyncResult {
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
    session: &mut super::ImapAnySession,
    account_id: &str,
    folder_id: &str,
    mailbox_name: &str,
    previous_folder: Option<&MailboxFolder>,
) -> Result<super::MailboxSelectResult, BackendError> {
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
                let known_uids = memory::store()
                    .mail()
                    .get_imap_uids_for_folder(account_id, folder_id);
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
        return Ok(super::MailboxSelectResult {
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
    Ok(super::MailboxSelectResult {
        mailbox,
        vanished_uids: Vec::new(),
    })
}

fn resolve_or_create_folder(account_id: &str, mailbox_name: &str) -> MailboxFolder {
    if let Ok(folders) = memory::store().mail().list_folders(account_id) {
        if let Some(folder) = folders
            .into_iter()
            .find(|folder| folder.imap_name.as_deref() == Some(mailbox_name))
        {
            return folder;
        }
    }

    let display_name = super::parse_ops::decode_imap_utf7(mailbox_name);
    let (kind, icon) = classify_folder(&display_name);
    MailboxFolder {
        id: format!("{}-{}", super::parse_ops::slug(mailbox_name), account_id),
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
    session: &mut super::ImapAnySession,
    mailbox_name: &str,
    uid_validity: u32,
    highest_modseq: u64,
    known_uids: &[u32],
) -> Result<super::MailboxSelectResult, BackendError> {
    let known_uid_set = compress_uid_set(known_uids);
    let command = format!(
        "SELECT {} (QRESYNC ({} {} {}))",
        super::parse_ops::quote_imap_string(mailbox_name),
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
            return Err(BackendError::internal("IMAP QRESYNC SELECT connection lost"));
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

    Ok(super::MailboxSelectResult {
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
