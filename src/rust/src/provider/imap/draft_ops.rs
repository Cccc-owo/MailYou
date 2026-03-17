use crate::models::{DraftMessage, MailAccount, MailIdentity, MailMessage, StoredAccountState};
use crate::protocol::BackendError;
use crate::provider::common::{
    base64_decode, decode_header_value, get_attachment_content_from_storage, strip_html_tags,
};
use crate::storage::{memory, persisted};

pub(super) const DRAFT_HEADER_IDENTITY: &str = "X-MailYou-Draft-Identity";
pub(super) const DRAFT_HEADER_REPLY: &str = "X-MailYou-Draft-In-Reply-To";
pub(super) const DRAFT_HEADER_FORWARD: &str = "X-MailYou-Draft-Forward-From";
pub(super) const DRAFT_HEADER_BCC: &str = "X-MailYou-Draft-Bcc";

pub(super) async fn materialize_remote_draft(
    account_id: &str,
    message: &MailMessage,
) -> Result<DraftMessage, BackendError> {
    let account = memory::store()
        .accounts()
        .get_account_state(account_id)
        .ok_or_else(|| BackendError::not_found("Account not found"))?
        .account;
    let raw = persisted::load_raw_email(&message.id).ok();
    let parsed = raw
        .as_ref()
        .and_then(|raw_email| mailparse::parse_mail(raw_email).ok());
    let selected_identity_id = parsed
        .as_ref()
        .and_then(|mail| parse_text_header(mail, DRAFT_HEADER_IDENTITY))
        .filter(|value| !value.is_empty())
        .or_else(|| {
            account
                .identities
                .iter()
                .find(|identity| identity.email.eq_ignore_ascii_case(&message.from_email))
                .map(|identity| identity.id.clone())
        });
    let mut attachments = Vec::with_capacity(message.attachments.len());
    for attachment in &message.attachments {
        let content = get_attachment_content_from_storage(account_id, &message.id, &attachment.id)?;
        attachments.push(crate::models::DraftAttachment {
            file_name: content.file_name,
            mime_type: content.mime_type,
            data_base64: content.data_base64,
        });
    }
    let bcc = parsed
        .as_ref()
        .and_then(|mail| {
            parse_text_header(mail, DRAFT_HEADER_BCC).or_else(|| parse_address_header(mail, "bcc"))
        })
        .unwrap_or_default();
    let in_reply_to_message_id = parsed
        .as_ref()
        .and_then(|mail| {
            parse_text_header(mail, DRAFT_HEADER_REPLY)
                .or_else(|| parse_text_header(mail, "in-reply-to"))
        })
        .filter(|value| !value.is_empty());
    let forward_from_message_id = parsed
        .as_ref()
        .and_then(|mail| parse_text_header(mail, DRAFT_HEADER_FORWARD))
        .filter(|value| !value.is_empty());

    Ok(DraftMessage {
        id: message.id.clone(),
        account_id: account_id.to_string(),
        selected_identity_id,
        to: message.to.join(", "),
        cc: message.cc.join(", "),
        bcc,
        subject: message.subject.clone(),
        body: message.body.clone(),
        in_reply_to_message_id,
        forward_from_message_id,
        attachments,
    })
}

pub(super) fn build_rfc822_message(
    state: &StoredAccountState,
    draft: &DraftMessage,
) -> Result<lettre::Message, BackendError> {
    use lettre::message::header::ContentType;
    use lettre::message::header::{HeaderName, HeaderValue, InReplyTo};
    use lettre::message::{Attachment, Mailbox, MultiPart, SinglePart};
    use lettre::Message;

    let identity = resolve_sender_identity(&state.account, draft.selected_identity_id.as_deref());
    let from: Mailbox = format!("{} <{}>", identity.name, identity.email)
        .parse()
        .map_err(|e| BackendError::validation(format!("Invalid sender address: {e}")))?;

    let mut builder = Message::builder().from(from);
    if let Some(identity_id) = draft
        .selected_identity_id
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        builder = builder.raw_header(HeaderValue::new(
            HeaderName::new_from_ascii_str(DRAFT_HEADER_IDENTITY),
            identity_id.clone(),
        ));
    }
    if let Some(reply_id) = draft
        .in_reply_to_message_id
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        builder = builder
            .header(InReplyTo::from(reply_id.clone()))
            .raw_header(HeaderValue::new(
                HeaderName::new_from_ascii_str(DRAFT_HEADER_REPLY),
                reply_id.clone(),
            ));
    }
    if let Some(forward_id) = draft
        .forward_from_message_id
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        builder = builder.raw_header(HeaderValue::new(
            HeaderName::new_from_ascii_str(DRAFT_HEADER_FORWARD),
            forward_id.clone(),
        ));
    }
    if !draft.bcc.trim().is_empty() {
        builder = builder.raw_header(HeaderValue::new(
            HeaderName::new_from_ascii_str(DRAFT_HEADER_BCC),
            draft.bcc.clone(),
        ));
    }
    if let Some(reply_to) = identity
        .reply_to
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        let reply_to_mailbox: Mailbox = reply_to
            .parse()
            .map_err(|e| BackendError::validation(format!("Invalid Reply-To address: {e}")))?;
        builder = builder.reply_to(reply_to_mailbox);
    }

    for recipient in draft.to.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        let to: Mailbox = recipient.parse().map_err(|e| {
            BackendError::validation(format!("Invalid recipient '{recipient}': {e}"))
        })?;
        builder = builder.to(to);
    }

    for recipient in draft.cc.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        let cc: Mailbox = recipient
            .parse()
            .map_err(|e| BackendError::validation(format!("Invalid CC '{recipient}': {e}")))?;
        builder = builder.cc(cc);
    }

    for recipient in draft
        .bcc
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        let bcc: Mailbox = recipient
            .parse()
            .map_err(|e| BackendError::validation(format!("Invalid BCC '{recipient}': {e}")))?;
        builder = builder.bcc(bcc);
    }

    let plain_text = strip_html_tags(&draft.body);
    let alternative = MultiPart::alternative()
        .singlepart(SinglePart::plain(plain_text))
        .singlepart(SinglePart::html(draft.body.clone()));

    let email = if draft.attachments.is_empty() {
        builder
            .subject(&draft.subject)
            .multipart(alternative)
            .map_err(|e| BackendError::internal(format!("Failed to build email: {e}")))?
    } else {
        let mut multipart = MultiPart::mixed().multipart(alternative);

        for att in &draft.attachments {
            let decoded = base64_decode(&att.data_base64).unwrap_or_default();
            let content_type =
                ContentType::parse(&att.mime_type).unwrap_or(ContentType::TEXT_PLAIN);
            let attachment = Attachment::new(att.file_name.clone()).body(decoded, content_type);
            multipart = multipart.singlepart(attachment);
        }

        builder
            .subject(&draft.subject)
            .multipart(multipart)
            .map_err(|e| BackendError::internal(format!("Failed to build email: {e}")))?
    };

    Ok(email)
}

pub(super) fn parse_text_header(
    mail: &mailparse::ParsedMail<'_>,
    header_name: &str,
) -> Option<String> {
    mail.headers
        .iter()
        .find(|header| header.get_key().eq_ignore_ascii_case(header_name))
        .map(|header| decode_header_value(header.get_value_raw()))
        .map(|value| value.trim().to_string())
}

pub(super) fn parse_address_header(
    mail: &mailparse::ParsedMail<'_>,
    header_name: &str,
) -> Option<String> {
    let header = mail
        .headers
        .iter()
        .find(|header| header.get_key().eq_ignore_ascii_case(header_name))?;
    let address_list = mailparse::addrparse_header(header).ok()?;
    let mut recipients = Vec::new();
    for address in address_list.iter() {
        flatten_mail_address(address, &mut recipients);
    }
    Some(recipients.join(", "))
}

fn flatten_mail_address(address: &mailparse::MailAddr, recipients: &mut Vec<String>) {
    match address {
        mailparse::MailAddr::Single(single) => push_single_address(single, recipients),
        mailparse::MailAddr::Group(group) => {
            for member in &group.addrs {
                push_single_address(member, recipients);
            }
        }
    }
}

fn push_single_address(single: &mailparse::SingleInfo, recipients: &mut Vec<String>) {
    if let Some(display_name) = single.display_name.as_ref().filter(|name| !name.is_empty()) {
        recipients.push(format!("{display_name} <{}>", single.addr));
    } else {
        recipients.push(single.addr.clone());
    }
}

fn resolve_sender_identity(account: &MailAccount, identity_id: Option<&str>) -> MailIdentity {
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
        .unwrap_or(MailIdentity {
            id: format!("identity-{}-default", account.id),
            name: account.name.clone(),
            email: account.email.clone(),
            reply_to: None,
            signature: None,
            is_default: true,
        })
}

#[cfg(test)]
mod tests {
    use super::{
        build_rfc822_message, parse_address_header, parse_text_header, DRAFT_HEADER_BCC,
        DRAFT_HEADER_FORWARD, DRAFT_HEADER_IDENTITY, DRAFT_HEADER_REPLY,
    };
    use crate::models::{
        AccountAuthMode, AccountConfig, AccountStatus, DraftAttachment, DraftMessage, MailAccount,
        MailIdentity, StoredAccountState,
    };

    fn fixture_state() -> StoredAccountState {
        StoredAccountState {
            account: MailAccount {
                id: "acc-1".into(),
                name: "Primary".into(),
                email: "primary@example.com".into(),
                provider: "imap-smtp".into(),
                incoming_protocol: "imap".into(),
                auth_mode: AccountAuthMode::Password,
                oauth_provider: None,
                oauth_source: None,
                color: "#5B8DEF".into(),
                initials: "PR".into(),
                unread_count: 0,
                status: AccountStatus::Connected,
                last_synced_at: "2026-03-17T00:00:00.000Z".into(),
                identities: vec![
                    MailIdentity {
                        id: "identity-default".into(),
                        name: "Primary".into(),
                        email: "primary@example.com".into(),
                        reply_to: None,
                        signature: None,
                        is_default: true,
                    },
                    MailIdentity {
                        id: "identity-alt".into(),
                        name: "Alt Sender".into(),
                        email: "alias@example.com".into(),
                        reply_to: Some("reply@example.com".into()),
                        signature: Some("<p>sig</p>".into()),
                        is_default: false,
                    },
                ],
            },
            config: AccountConfig {
                auth_mode: AccountAuthMode::Password,
                incoming_protocol: "imap".into(),
                incoming_host: "imap.example.com".into(),
                incoming_port: 993,
                outgoing_host: "smtp.example.com".into(),
                outgoing_port: 465,
                username: "primary@example.com".into(),
                password: "secret".into(),
                use_tls: true,
                oauth_provider: None,
                oauth_source: None,
                access_token: String::new(),
                refresh_token: String::new(),
                token_expires_at: String::new(),
            },
        }
    }

    fn fixture_draft() -> DraftMessage {
        DraftMessage {
            id: "draft-1".into(),
            account_id: "acc-1".into(),
            selected_identity_id: Some("identity-alt".into()),
            to: "Alice <alice@example.com>".into(),
            cc: "Bob <bob@example.com>".into(),
            bcc: "Secret <secret@example.com>".into(),
            subject: "Subject".into(),
            body: "<p>Hello</p>".into(),
            in_reply_to_message_id: Some("<reply-id@example.com>".into()),
            forward_from_message_id: Some("forward-source-1".into()),
            attachments: vec![DraftAttachment {
                file_name: "hello.txt".into(),
                mime_type: "text/plain".into(),
                data_base64: "aGVsbG8=".into(),
            }],
        }
    }

    #[test]
    fn build_rfc822_message_embeds_mailyou_draft_headers() {
        let state = fixture_state();
        let draft = fixture_draft();
        let message = build_rfc822_message(&state, &draft).expect("message should build");
        let raw = message.formatted();
        let parsed = mailparse::parse_mail(&raw).expect("message should parse");

        assert_eq!(
            parse_text_header(&parsed, DRAFT_HEADER_IDENTITY).as_deref(),
            Some("identity-alt")
        );
        assert_eq!(
            parse_text_header(&parsed, DRAFT_HEADER_REPLY).as_deref(),
            Some("<reply-id@example.com>")
        );
        assert_eq!(
            parse_text_header(&parsed, DRAFT_HEADER_FORWARD).as_deref(),
            Some("forward-source-1")
        );
        assert_eq!(
            parse_text_header(&parsed, "in-reply-to").as_deref(),
            Some("<reply-id@example.com>")
        );
    }

    #[test]
    fn build_rfc822_message_preserves_bcc_header_for_remote_draft_restore() {
        let state = fixture_state();
        let draft = fixture_draft();
        let message = build_rfc822_message(&state, &draft).expect("message should build");
        let raw = message.formatted();
        let parsed = mailparse::parse_mail(&raw).expect("message should parse");

        assert_eq!(
            parse_text_header(&parsed, DRAFT_HEADER_BCC).as_deref(),
            Some("Secret <secret@example.com>")
        );
        assert_eq!(
            parse_address_header(&parsed, "to").as_deref(),
            Some("Alice <alice@example.com>")
        );
        assert_eq!(
            parse_address_header(&parsed, "cc").as_deref(),
            Some("Bob <bob@example.com>")
        );
    }

    #[test]
    fn build_rfc822_message_uses_selected_identity_and_reply_to() {
        let state = fixture_state();
        let draft = fixture_draft();
        let message = build_rfc822_message(&state, &draft).expect("message should build");
        let raw = message.formatted();
        let parsed = mailparse::parse_mail(&raw).expect("message should parse");

        assert_eq!(
            parse_address_header(&parsed, "from").as_deref(),
            Some("Alt Sender <alias@example.com>")
        );
        assert_eq!(
            parse_address_header(&parsed, "reply-to").as_deref(),
            Some("reply@example.com")
        );
    }
}
