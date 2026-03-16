use crate::models::{
    AccountAuthMode, AccountConfig, AccountSetupDraft, AccountStatus, MailAccount, StoredAccountState,
};

pub fn seeded_account_states() -> Vec<StoredAccountState> {
    vec![
        StoredAccountState {
            account: MailAccount {
                id: "acc-work".into(),
                name: "MailYou Work".into(),
                email: "hello@mailyou.dev".into(),
                provider: "Fastmail".into(),
                incoming_protocol: "imap".into(),
                auth_mode: AccountAuthMode::Password,
                oauth_provider: None,
                oauth_source: None,
                color: "#6D5DFB".into(),
                initials: "MY".into(),
                unread_count: 6,
                status: AccountStatus::Syncing,
                last_synced_at: "2026-03-12T09:41:00.000Z".into(),
            },
            config: AccountConfig {
                auth_mode: AccountAuthMode::Password,
                incoming_protocol: "imap".into(),
                incoming_host: "imap.fastmail.com".into(),
                incoming_port: 993,
                outgoing_host: "smtp.fastmail.com".into(),
                outgoing_port: 465,
                username: "hello@mailyou.dev".into(),
                password: "demo-password".into(),
                use_tls: true,
                oauth_provider: None,
                oauth_source: None,
                access_token: String::new(),
                refresh_token: String::new(),
                token_expires_at: String::new(),
            },
        },
        StoredAccountState {
            account: MailAccount {
                id: "acc-personal".into(),
                name: "Personal".into(),
                email: "iscccc@example.com".into(),
                provider: "Gmail".into(),
                incoming_protocol: "imap".into(),
                auth_mode: AccountAuthMode::Password,
                oauth_provider: None,
                oauth_source: None,
                color: "#0F9D58".into(),
                initials: "IP".into(),
                unread_count: 3,
                status: AccountStatus::Connected,
                last_synced_at: "2026-03-12T09:27:00.000Z".into(),
            },
            config: AccountConfig {
                auth_mode: AccountAuthMode::Password,
                incoming_protocol: "imap".into(),
                incoming_host: "imap.gmail.com".into(),
                incoming_port: 993,
                outgoing_host: "smtp.gmail.com".into(),
                outgoing_port: 465,
                username: "iscccc@example.com".into(),
                password: "demo-password".into(),
                use_tls: true,
                oauth_provider: None,
                oauth_source: None,
                access_token: String::new(),
                refresh_token: String::new(),
                token_expires_at: String::new(),
            },
        },
    ]
}

pub fn config_from_draft(draft: &AccountSetupDraft) -> AccountConfig {
    AccountConfig {
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
    }
}
