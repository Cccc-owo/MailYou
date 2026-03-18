pub mod account;
pub mod contacts;
pub mod draft;
pub mod mailbox;
pub mod sync;

pub use account::{
    AccountAuthMode, AccountConfig, AccountSetupDraft, AccountStatus, MailAccount, MailIdentity,
    OAuthProviderAvailability, OAuthSource, StoredAccountState,
};
pub use contacts::{Contact, ContactGroup};
pub use draft::{DraftAttachment, DraftMessage};
pub use mailbox::{
    AttachmentContent, AttachmentMeta, MailFolderKind, MailLabel, MailMessage, MailThread,
    MailboxBundle, MailboxFolder,
};
pub use sync::{AccountQuota, SyncStatus};
