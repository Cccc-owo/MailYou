# MailYou Environment

This document lists the environment variables currently used by MailYou and their defaults.

## App runtime

| Variable | Default | Used by | Notes |
| --- | --- | --- | --- |
| `MAILYOU_TARGET` | `electron` | Vite build | Set to `web` for the web target. |
| `VITE_DEV_SERVER_URL` | unset | Electron dev runtime | When set, Electron loads the Vite dev server and starts the Rust backend via `cargo run`. |
| `MAILYOU_OZONE_PLATFORM_HINT` | `auto` | Electron on Linux | Passed to Electron as `--ozone-platform-hint`. |
| `MAILYOU_ENABLE_DEV_PROTOCOL_CLIENT` | `false` | Electron on Linux dev | Enables `mailyou://` protocol registration during Linux dev runs. Disabled by default to avoid noisy `xdg-mime` errors. |
| `MAILYOU_DATA_DIR` | platform data dir | Rust backend storage | Overrides the base data directory before MailYou appends its own storage folders. |

## OAuth proxy

| Variable | Default | Used by | Notes |
| --- | --- | --- | --- |
| `MAILYOU_OAUTH_PROXY_URL` | `https://oauth2-proxy.iscccc.cc` | Electron OAuth, Rust token refresh | Hidden from user-facing settings. |
| `MAILYOU_OAUTH_PROXY_TOKEN` | `Lu6WVgtL31TkaXWVeVBIaB8T8CsU3jMfXoxbpomAuas5hF5wpOx5IWfdUiokkc5G` | Electron OAuth, Rust token refresh | Optional override for the public desktop proxy token. |

The desktop app also uses these built-in defaults:

| Key | Value |
| --- | --- |
| Callback protocol | `mailyou://oauth/callback` |
| Proxy bearer token | `Lu6WVgtL31TkaXWVeVBIaB8T8CsU3jMfXoxbpomAuas5hF5wpOx5IWfdUiokkc5G` |

`MAILYOU_OAUTH_PROXY_TOKEN` is a client-side override, not a secret storage mechanism. If you distribute it with the desktop app, users can extract it.

## Direct OAuth provider credentials

### Gmail

| Variable | Required for |
| --- | --- |
| `GMAIL_CLIENT_ID` | Direct OAuth auth + refresh |
| `GMAIL_CLIENT_SECRET` | Direct OAuth auth + refresh |
| `GMAIL_REDIRECT_URI` | Direct OAuth browser auth |

### Outlook

| Variable | Required for |
| --- | --- |
| `OUTLOOK_CLIENT_ID` | Direct OAuth auth + refresh |
| `OUTLOOK_CLIENT_SECRET` | Direct OAuth auth + refresh |
| `OUTLOOK_REDIRECT_URI` | Direct OAuth browser auth |

## Built-in mail provider presets

The add-account flow uses shared built-in presets from `src/config/mailProviders.ts`:

- OAuth-capable: `gmail`, `gmail.com`, `googlemail.com`, `outlook`, `outlook.com`, `hotmail.com`, `live.com`, `icloud`, `icloud.com`, `me.com`, `mac.com`
- Password-only: `qq.com`, `foxmail.com`, `163.com`, `126.com`, `yeah.net`

If a domain does not match a preset, MailYou falls back to manual IMAP/SMTP configuration.
