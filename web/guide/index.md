# MailYou

MailYou is a local-first desktop mail client built for a calmer daily workflow.

## Built for desktop workflows

MailYou uses Vue for the interface, Electron for desktop integration, and Rust for mail access and local persistence. The goal is simple: keep the app comfortable to use while keeping mail data close to the device.

## Account setup

MailYou tries to keep setup short:

- It starts from the email address.
- It picks a provider preset when the domain is known.
- It only shows OAuth when that provider actually supports OAuth.
- It keeps server details hidden unless preset data is missing or advanced mode is enabled.

## Authentication

MailYou supports two OAuth paths:

- Direct provider OAuth with local `client_id` and `client_secret`
- MailYou OAuth proxy with a built-in default URL and desktop token

For providers without OAuth, MailYou falls back to standard password-based setup.

## Privacy direction

MailYou is designed around local desktop storage. It does not position itself as a hosted mailbox or cloud sync service.

Legal documents are published on this website:

- [Privacy Policy](/legal/privacy-policy)
- [Terms of Service](/legal/terms-of-service)
