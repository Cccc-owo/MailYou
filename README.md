[English](README.md) | [中文](README.zh-CN.md)

# MailYou

MailYou is a desktop-first mail client built with Vue, Electron, and a Rust mail backend.

It supports password-based IMAP/SMTP accounts, OAuth-based sign-in for selected providers, local desktop storage, and packaged Linux/Windows builds.

## Features

- Desktop app built with Electron and Vue
- Rust backend for mail access and local storage
- IMAP / SMTP account setup with built-in provider presets
- OAuth support for Gmail, Outlook, and iCloud through direct credentials or the MailYou OAuth proxy
- Linux packaging targets including AppImage, deb, rpm, and zip

## Stack

- Vue 3
- Vuetify
- Electron
- Rust
- Vite
- Pinia

## Requirements

- Node.js
- npm
- Rust toolchain with `cargo`

## Development

Install dependencies:

```bash
npm install
```

Start the desktop app in development mode:

```bash
npm run dev
```

Start the web target:

```bash
npm run dev:web
```

## Build

Build the desktop app and make dist:

```bash
npm run dist
```

Build Linux packages:

```bash
npm run dist:appimage
npm run dist:deb
npm run dist:rpm
```

Build Windows packages:

```bash
npm run dist:win
```

## OAuth

MailYou supports two OAuth paths:

- Direct OAuth using local provider credentials from environment variables
- Proxy OAuth using the MailYou OAuth proxy

The desktop app uses these built-in defaults:

- Proxy URL: `https://oauth2-proxy.iscccc.cc`
- Callback protocol: `mailyou://oauth/callback`
- Proxy token: built in, with environment override support

If direct OAuth is not configured for a provider, MailYou can fall back to the proxy flow.

## Configuration

Configuration is documented here:

- [Environment variables](docs/environment.md)

Current shared config entry points:

- [Mail provider presets](src/config/mailProviders.ts)
- [OAuth defaults](src/config/oauth.ts)
- [Runtime defaults](src/config/runtime.ts)

## Provider Presets

Built-in presets currently cover:

- OAuth-capable domains: Gmail, Outlook, iCloud
- Password-based domains: QQ, Foxmail, 163, 126, Yeah

Unknown domains fall back to manual IMAP / SMTP setup.

## Project Layout

```text
src/
  config/      Shared app configuration
  electron/    Electron main process and desktop integrations
  rust/        Rust mail backend
  views/       Vue screens
docs/          Project documentation
```

## Notes For Packagers

- OAuth proxy access works out of the box with the built-in public desktop token.
- Packagers can override the proxy URL or proxy token with environment variables.
- Linux dev runs do not register the `mailyou://` protocol handler by default unless explicitly enabled.
