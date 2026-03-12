use crate::models::{
    AccountConfig, AccountSetupDraft, AccountStatus, MailAccount, StoredAccountState,
};

pub fn seeded_account_states() -> Vec<StoredAccountState> {
    vec![
        StoredAccountState {
            account: MailAccount {
                id: "acc-work".into(),
                name: "MailStack Work".into(),
                email: "hello@mailstack.dev".into(),
                provider: "Fastmail".into(),
                color: "#6D5DFB".into(),
                initials: "MW".into(),
                unread_count: 6,
                status: AccountStatus::Syncing,
                last_synced_at: "2026-03-12T09:41:00.000Z".into(),
            },
            config: AccountConfig {
                incoming_host: "imap.fastmail.com".into(),
                incoming_port: 993,
                outgoing_host: "smtp.fastmail.com".into(),
                outgoing_port: 465,
                username: "hello@mailstack.dev".into(),
                password: "demo-password".into(),
                use_tls: true,
            },
        },
        StoredAccountState {
            account: MailAccount {
                id: "acc-personal".into(),
                name: "Personal".into(),
                email: "iscccc@example.com".into(),
                provider: "Gmail".into(),
                color: "#0F9D58".into(),
                initials: "IP".into(),
                unread_count: 3,
                status: AccountStatus::Connected,
                last_synced_at: "2026-03-12T09:27:00.000Z".into(),
            },
            config: AccountConfig {
                incoming_host: "imap.gmail.com".into(),
                incoming_port: 993,
                outgoing_host: "smtp.gmail.com".into(),
                outgoing_port: 465,
                username: "iscccc@example.com".into(),
                password: "demo-password".into(),
                use_tls: true,
            },
        },
    ]
}

pub fn config_from_draft(draft: &AccountSetupDraft) -> AccountConfig {
    AccountConfig {
        incoming_host: draft.incoming_host.clone(),
        incoming_port: draft.incoming_port,
        outgoing_host: draft.outgoing_host.clone(),
        outgoing_port: draft.outgoing_port,
        username: draft.username.clone(),
        password: draft.password.clone(),
        use_tls: draft.use_tls,
    }
}
