use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::models::MailFolderKind;
use crate::protocol::BackendMessage;
use crate::provider::imap::{sync_mailbox_incremental, wait_for_mailbox_change, IdleMailboxChange};
use crate::storage::memory;

#[derive(Clone, Default)]
pub struct RealtimeController {
    tasks: Arc<Mutex<HashMap<String, JoinHandle<()>>>>,
}

#[derive(Clone, Copy)]
struct RealtimeProfile {
    watch_junk: bool,
    idle_timeout: Duration,
    base_backoff: Duration,
    max_backoff: Duration,
}

impl RealtimeController {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reconcile(&self, tx: mpsc::UnboundedSender<BackendMessage>) {
        let Ok(accounts) = memory::store().accounts().list_accounts() else {
            return;
        };

        let mut desired = HashSet::new();
        let mut task_specs = Vec::new();

        for account in accounts
            .into_iter()
            .filter(|account| account.incoming_protocol == "imap")
        {
            let profile = realtime_profile(&account.provider);
            for mailbox_name in resolve_watched_mailboxes(&account.id, profile.watch_junk) {
                let task_key = format!("{}\n{}", account.id, mailbox_name);
                desired.insert(task_key.clone());
                task_specs.push((task_key, account.id.clone(), mailbox_name, profile));
            }
        }

        let mut tasks = self.tasks.lock().unwrap();
        let existing_ids: Vec<String> = tasks.keys().cloned().collect();

        for task_id in existing_ids {
            if desired.contains(&task_id) {
                continue;
            }

            if let Some(handle) = tasks.remove(&task_id) {
                handle.abort();
            }
        }

        for (task_key, account_id, mailbox_name, profile) in task_specs {
            if tasks.contains_key(&task_key) {
                continue;
            }

            let tx = tx.clone();
            let handle = tokio::spawn(async move {
                run_realtime_loop(account_id, mailbox_name, profile, tx).await;
            });
            tasks.insert(task_key, handle);
        }
    }

    pub fn shutdown(&self) {
        let mut tasks = self.tasks.lock().unwrap();
        for (_, handle) in tasks.drain() {
            handle.abort();
        }
    }
}

async fn run_realtime_loop(
    account_id: String,
    mailbox_name: String,
    profile: RealtimeProfile,
    tx: mpsc::UnboundedSender<BackendMessage>,
) {
    let mut backoff = profile.base_backoff;

    loop {
        match idle_until_mailbox_changes(&account_id, &mailbox_name, profile.idle_timeout).await {
            Ok(IdleMailboxChange::Changed) => {
                if let Err(error) = sync_mailbox_incremental(&account_id, &mailbox_name).await {
                    eprintln!(
                        "[idle] incremental sync failed for {account_id} {mailbox_name}: {}",
                        error.message
                    );
                } else if let Ok(event) =
                    BackendMessage::mailbox_changed(account_id.clone(), "idle")
                {
                    let _ = tx.send(event);
                }
                backoff = profile.base_backoff;
            }
            Ok(IdleMailboxChange::Vanished(uids)) => {
                if let Some(folder_id) = resolve_folder_id(&account_id, &mailbox_name) {
                    if let Err(error) =
                        memory::store()
                            .mail()
                            .remove_messages_by_imap_uids(&account_id, &folder_id, &uids)
                    {
                        eprintln!(
                            "[idle] vanished prune failed for {account_id} {mailbox_name}: {}",
                            error.message
                        );
                    }
                }

                if let Err(error) = sync_mailbox_incremental(&account_id, &mailbox_name).await {
                    eprintln!(
                        "[idle] incremental sync after vanished failed for {account_id} {mailbox_name}: {}",
                        error.message
                    );
                } else if let Ok(event) =
                    BackendMessage::mailbox_changed(account_id.clone(), "idle")
                {
                    let _ = tx.send(event);
                }
                backoff = profile.base_backoff;
            }
            Ok(IdleMailboxChange::Timeout) => {
                backoff = profile.base_backoff;
            }
            Err(error) => {
                eprintln!(
                    "[idle] watcher failed for {account_id} {mailbox_name}: {}",
                    error.message
                );
                tokio::time::sleep(backoff).await;
                backoff = std::cmp::min(backoff.saturating_mul(2), profile.max_backoff);
            }
        }
    }
}

async fn idle_until_mailbox_changes(
    account_id: &str,
    mailbox_name: &str,
    idle_timeout: Duration,
) -> Result<IdleMailboxChange, crate::protocol::BackendError> {
    let account_state = memory::store()
        .accounts()
        .get_account_state(account_id)
        .ok_or_else(|| crate::protocol::BackendError::not_found("Account not found"))?;
    wait_for_mailbox_change(&account_state, mailbox_name, idle_timeout).await
}

fn resolve_folder_id(account_id: &str, mailbox_name: &str) -> Option<String> {
    memory::store().mail().list_folders(account_id).ok().and_then(|folders| {
        folders
            .into_iter()
            .find(|folder| folder.imap_name.as_deref() == Some(mailbox_name))
            .map(|folder| folder.id)
    })
}

fn resolve_watched_mailboxes(account_id: &str, watch_junk: bool) -> Vec<String> {
    let Ok(folders) = memory::store().mail().list_folders(account_id) else {
        return vec!["INBOX".into()];
    };

    let mut watched = Vec::new();
    let mut seen = HashSet::new();

    for folder in folders {
        let should_watch = matches!(folder.kind, MailFolderKind::Inbox)
            || (watch_junk && matches!(folder.kind, MailFolderKind::Junk));
        if !should_watch {
            continue;
        }

        if let Some(imap_name) = folder.imap_name {
            if seen.insert(imap_name.clone()) {
                watched.push(imap_name);
            }
        }
    }

    if watched.is_empty() {
        watched.push("INBOX".into());
    }

    watched
}

fn realtime_profile(provider: &str) -> RealtimeProfile {
    let normalized = provider.trim().to_lowercase();

    if normalized.contains("qq")
        || normalized.contains("netease")
        || normalized.contains("163")
        || normalized.contains("126")
    {
        return RealtimeProfile {
            watch_junk: true,
            idle_timeout: Duration::from_secs(60 * 8),
            base_backoff: Duration::from_secs(3),
            max_backoff: Duration::from_secs(90),
        };
    }

    if normalized.contains("outlook")
        || normalized.contains("hotmail")
        || normalized.contains("live")
    {
        return RealtimeProfile {
            watch_junk: true,
            idle_timeout: Duration::from_secs(60 * 8),
            base_backoff: Duration::from_secs(5),
            max_backoff: Duration::from_secs(120),
        };
    }

    if normalized.contains("gmail") || normalized.contains("google") {
        return RealtimeProfile {
            watch_junk: false,
            idle_timeout: Duration::from_secs(60 * 9),
            base_backoff: Duration::from_secs(5),
            max_backoff: Duration::from_secs(120),
        };
    }

    RealtimeProfile {
        watch_junk: true,
        idle_timeout: Duration::from_secs(60 * 8),
        base_backoff: Duration::from_secs(5),
        max_backoff: Duration::from_secs(120),
    }
}
