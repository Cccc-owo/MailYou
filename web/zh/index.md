---
layout: home

hero:
  name: MailYou
  text: 一个更舒适的本地优先桌面邮箱客户端
  tagline: MailYou 将桌面体验、本地存储、服务商预设与现代 OAuth 支持结合在一起。
  image:
    src: /logo.svg
    alt: MailYou
  actions:
    - theme: brand
      text: 快速了解
      link: /zh/guide/
    - theme: alt
      text: English
      link: /

features:
  - title: 面向桌面
    details: 使用 Vue、Electron 与 Rust 构建，服务于真正的桌面邮件工作流，而不是浏览器中的网页邮箱体验。
  - title: 添加邮箱更直接
    details: 常见服务商已预设完成，只有在需要手动配置时才展开更详细的服务器选项。
  - title: OAuth 已就绪
    details: 对支持的服务商，可使用本地直连 OAuth，也可使用内置的 MailYou OAuth 代理。
  - title: 本地优先
    details: 邮件数据、草稿与账户状态以本地设备为主要存储位置。
---

## MailYou 是什么

MailYou 是一个本地优先的桌面邮箱客户端，重点在于稳定的账户接入、更高效的配置流程，以及更舒适的日常使用体验。

应用界面基于 Vue 与 Electron 构建，邮件访问与本地持久化能力由 Rust 后端负责。

## 当前能力

- 桌面应用，不是网页邮箱服务
- 账号信息保存在本机
- 内置 Gmail、Outlook、iCloud、QQ 邮箱、163、126、Yeah、Foxmail 等预设
- OAuth 可走本地凭证，也可走 MailYou 公共代理
