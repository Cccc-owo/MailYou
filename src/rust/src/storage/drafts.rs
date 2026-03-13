use crate::models::DraftMessage;

pub fn seeded_drafts() -> Vec<DraftMessage> {
    vec![DraftMessage {
        id: "draft-1".into(),
        account_id: "acc-work".into(),
        to: "infra@mailyou.dev".into(),
        cc: "".into(),
        bcc: "".into(),
        subject: "Sync engine handoff notes".into(),
        body: "Starting a short handoff doc for the sync engine boundaries and retry model.".into(),
        in_reply_to_message_id: None,
        forward_from_message_id: None,
    }]
}
