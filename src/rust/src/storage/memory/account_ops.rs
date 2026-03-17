use crate::models::{
    AccountSetupDraft, AccountStatus, MailAccount, MailIdentity, StoredAccountState, SyncStatus,
};
use crate::protocol::BackendError;
use crate::storage::{accounts, mailbox, memory, sync};

pub(crate) fn create_account_without_test(
    draft: AccountSetupDraft,
) -> Result<MailAccount, BackendError> {
    let mut state = memory::lock_state();
    let display_name = draft.display_name.trim();
    let base_name = if display_name.is_empty() {
        draft.email.trim()
    } else {
        display_name
    };

    let initials = base_name
        .split_whitespace()
        .filter_map(|part| part.chars().next())
        .take(2)
        .map(|character| character.to_uppercase().collect::<String>())
        .collect::<String>();

    let account_id = state.unique_account_id(&draft.email);
    let last_synced_at = memory::current_timestamp();
    let config = accounts::config_from_draft(&draft);
    let identities = normalize_identities(&account_id, &draft.identities, base_name, &draft.email);
    let account = MailAccount {
        id: account_id.clone(),
        name: base_name.to_string(),
        email: draft.email.clone(),
        provider: draft.provider.clone(),
        incoming_protocol: draft.incoming_protocol.clone(),
        auth_mode: draft.auth_mode.clone(),
        oauth_provider: draft.oauth_provider.clone(),
        oauth_source: draft.oauth_source.clone(),
        color: "#5B8DEF".into(),
        initials: if initials.is_empty() {
            "NA".into()
        } else {
            initials
        },
        unread_count: 0,
        status: AccountStatus::Connected,
        last_synced_at: last_synced_at.clone(),
        identities,
    };

    state.insert_account_state(StoredAccountState {
        account: account.clone(),
        config,
    });
    state
        .folders
        .splice(0..0, mailbox::default_folders_for_account(&account_id));
    state.sync_statuses.insert(
        account_id.clone(),
        sync::initial_sync_status(&account_id, &last_synced_at),
    );
    state.persist()?;

    Ok(account)
}

pub(crate) fn get_account_state(account_id: &str) -> Option<StoredAccountState> {
    memory::lock_state()
        .account_states
        .iter()
        .find(|s| s.account.id == account_id)
        .cloned()
}

pub(crate) fn get_account_config(account_id: &str) -> Result<AccountSetupDraft, BackendError> {
    let state = memory::lock_state();
    let account_state = state
        .account_states
        .iter()
        .find(|s| s.account.id == account_id)
        .ok_or_else(|| BackendError::not_found("Account not found"))?;

    Ok(AccountSetupDraft {
        display_name: account_state.account.name.clone(),
        email: account_state.account.email.clone(),
        provider: account_state.account.provider.clone(),
        auth_mode: account_state.config.auth_mode.clone(),
        incoming_protocol: account_state.config.incoming_protocol.clone(),
        incoming_host: account_state.config.incoming_host.clone(),
        incoming_port: account_state.config.incoming_port,
        outgoing_host: account_state.config.outgoing_host.clone(),
        outgoing_port: account_state.config.outgoing_port,
        username: account_state.config.username.clone(),
        password: account_state.config.password.clone(),
        use_tls: account_state.config.use_tls,
        oauth_provider: account_state.config.oauth_provider.clone(),
        oauth_source: account_state.config.oauth_source.clone(),
        access_token: account_state.config.access_token.clone(),
        refresh_token: account_state.config.refresh_token.clone(),
        token_expires_at: account_state.config.token_expires_at.clone(),
        identities: account_state.account.identities.clone(),
    })
}

pub(crate) fn update_account(
    account_id: &str,
    draft: AccountSetupDraft,
) -> Result<MailAccount, BackendError> {
    let mut state = memory::lock_state();
    let account_state = state
        .account_states
        .iter_mut()
        .find(|s| s.account.id == account_id)
        .ok_or_else(|| BackendError::not_found("Account not found"))?;

    let display_name = draft.display_name.trim();
    let base_name = if display_name.is_empty() {
        draft.email.trim()
    } else {
        display_name
    };

    let initials = base_name
        .split_whitespace()
        .filter_map(|part| part.chars().next())
        .take(2)
        .map(|character| character.to_uppercase().collect::<String>())
        .collect::<String>();

    account_state.account.name = base_name.to_string();
    account_state.account.email = draft.email.clone();
    account_state.account.provider = draft.provider.clone();
    account_state.account.incoming_protocol = draft.incoming_protocol.clone();
    account_state.account.auth_mode = draft.auth_mode.clone();
    account_state.account.oauth_provider = draft.oauth_provider.clone();
    account_state.account.oauth_source = draft.oauth_source.clone();
    account_state.account.initials = if initials.is_empty() {
        "NA".into()
    } else {
        initials
    };
    account_state.account.identities =
        normalize_identities(account_id, &draft.identities, base_name, &draft.email);
    account_state.config = accounts::config_from_draft(&draft);

    let updated = account_state.account.clone();
    state.persist()?;
    Ok(updated)
}

pub(crate) fn update_account_oauth_tokens(
    account_id: &str,
    access_token: &str,
    refresh_token: &str,
    expires_at: &str,
) -> Result<(), BackendError> {
    let mut state = memory::lock_state();
    let account_state = state
        .account_states
        .iter_mut()
        .find(|s| s.account.id == account_id)
        .ok_or_else(|| BackendError::not_found("Account not found"))?;

    account_state.config.access_token = access_token.to_string();
    account_state.config.refresh_token = refresh_token.to_string();
    account_state.config.token_expires_at = expires_at.to_string();
    state.persist()
}

pub(crate) fn finish_sync(account_id: &str, timestamp: &str) -> Result<SyncStatus, BackendError> {
    let mut state = memory::lock_state();

    let status = SyncStatus {
        account_id: account_id.into(),
        state: "idle".into(),
        message: "Sync completed successfully".into(),
        updated_at: timestamp.into(),
    };

    if let Some(account_state) = state
        .account_states
        .iter_mut()
        .find(|s| s.account.id == account_id)
    {
        account_state.account.last_synced_at = timestamp.into();
        account_state.account.status = AccountStatus::Connected;
    }

    state
        .sync_statuses
        .insert(account_id.to_string(), status.clone());
    state.persist()?;
    Ok(status)
}

pub(crate) fn default_identity(account_id: &str, name: &str, email: &str) -> MailIdentity {
    MailIdentity {
        id: format!("identity-{account_id}-default"),
        name: name.into(),
        email: email.into(),
        reply_to: None,
        signature: None,
        is_default: true,
    }
}

pub(crate) fn normalize_identities(
    account_id: &str,
    identities: &[MailIdentity],
    fallback_name: &str,
    fallback_email: &str,
) -> Vec<MailIdentity> {
    let mut seen_ids = std::collections::HashSet::new();
    let mut normalized = identities
        .iter()
        .enumerate()
        .filter_map(|(index, identity)| {
            let email = identity.email.trim();
            if email.is_empty() {
                return None;
            }

            let name = identity.name.trim();
            let mut id = if identity.id.trim().is_empty() {
                format!("identity-{account_id}-{}", index + 1)
            } else {
                identity.id.trim().to_string()
            };
            if !seen_ids.insert(id.clone()) {
                id = format!("identity-{account_id}-{}", index + 1);
                seen_ids.insert(id.clone());
            }
            let reply_to = identity
                .reply_to
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned);
            let signature = identity
                .signature
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned);

            Some(MailIdentity {
                id,
                name: if name.is_empty() {
                    fallback_name.to_string()
                } else {
                    name.to_string()
                },
                email: email.to_string(),
                reply_to,
                signature,
                is_default: identity.is_default,
            })
        })
        .collect::<Vec<_>>();

    if normalized.is_empty() {
        return vec![default_identity(account_id, fallback_name, fallback_email)];
    }

    let default_index = normalized
        .iter()
        .position(|identity| identity.is_default)
        .unwrap_or(0);
    for (index, identity) in normalized.iter_mut().enumerate() {
        identity.is_default = index == default_index;
    }

    normalized
}

pub(crate) fn resolve_identity(account: &MailAccount, identity_id: Option<&str>) -> MailIdentity {
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
        .unwrap_or_else(|| default_identity(&account.id, &account.name, &account.email))
}
