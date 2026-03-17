use crate::models::{MailFolderKind, MailboxFolder};

pub fn default_folders_for_account(account_id: &str) -> Vec<MailboxFolder> {
    vec![
        folder(
            &format!("inbox-{account_id}"),
            account_id,
            "Inbox",
            MailFolderKind::Inbox,
            0,
            0,
            "mdi-inbox-arrow-down",
        ),
        folder(
            &format!("starred-{account_id}"),
            account_id,
            "Starred",
            MailFolderKind::Starred,
            0,
            0,
            "mdi-star-outline",
        ),
        folder(
            &format!("sent-{account_id}"),
            account_id,
            "Sent",
            MailFolderKind::Sent,
            0,
            0,
            "mdi-send-outline",
        ),
        folder(
            &format!("drafts-{account_id}"),
            account_id,
            "Drafts",
            MailFolderKind::Drafts,
            0,
            0,
            "mdi-file-document-edit-outline",
        ),
        folder(
            &format!("archive-{account_id}"),
            account_id,
            "Archive",
            MailFolderKind::Archive,
            0,
            0,
            "mdi-archive-outline",
        ),
        folder(
            &format!("trash-{account_id}"),
            account_id,
            "Trash",
            MailFolderKind::Trash,
            0,
            0,
            "mdi-delete-outline",
        ),
        folder(
            &format!("junk-{account_id}"),
            account_id,
            "Junk",
            MailFolderKind::Junk,
            0,
            0,
            "mdi-alert-circle-outline",
        ),
    ]
}

fn folder(
    id: &str,
    account_id: &str,
    name: &str,
    kind: MailFolderKind,
    unread_count: u32,
    total_count: u32,
    icon: &str,
) -> MailboxFolder {
    MailboxFolder {
        id: id.into(),
        account_id: account_id.into(),
        name: name.into(),
        kind,
        unread_count,
        total_count,
        icon: icon.into(),
        imap_name: None,
        imap_uid_validity: None,
        imap_uid_next: None,
        imap_highest_modseq: None,
    }
}
