---
layout: home

hero:
  name: MailYou
  text: A refined local-first mail client for desktop
  tagline: MailYou combines a comfortable desktop experience with local storage, provider presets, and modern OAuth support.
  image:
    src: /logo.svg
    alt: MailYou
  actions:
    - theme: brand
      text: Quick Overview
      link: /guide/
    - theme: alt
      text: 中文介绍
      link: /zh/

features:
  - title: Desktop-first
    details: Built for desktop use with Vue, Electron, and a Rust backend rather than a browser-based mail workflow.
  - title: Fast account setup
    details: Common providers are preconfigured, while advanced server options remain available when manual setup is required.
  - title: OAuth ready
    details: Supports direct OAuth credentials as well as the built-in MailYou OAuth proxy for supported providers.
  - title: Local-first storage
    details: Mail data, drafts, and account state are designed to remain primarily on the local device.
---

## What MailYou is

MailYou is a local-first desktop mail client focused on stable account access, efficient setup, and a more comfortable daily workflow.

The application is built with Vue and Electron on the desktop side, with Rust handling mail access and local persistence.

## Current scope

- Desktop app, not a webmail service
- Local account storage on your machine
- Presets for Gmail, Outlook, iCloud, QQ Mail, 163, 126, Yeah, and Foxmail
- OAuth flow through direct credentials or the MailYou public proxy
