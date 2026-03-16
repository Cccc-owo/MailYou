[English](README.md) | [中文](README.zh-CN.md)

# MailYou

MailYou 是一个面向桌面的邮件客户端，使用 Vue、Electron 和 Rust 邮件后端构建。

支持基于密码的 IMAP/SMTP 邮箱、部分服务商的 OAuth 登录、本地桌面存储，以及 Linux / Windows 打包发布。

## 特性

- 基于 Electron 和 Vue 的桌面应用
- 使用 Rust 后端处理邮件访问和本地存储
- 支持 IMAP / SMTP 邮箱接入，并内置常见服务商预设
- 支持 Gmail、Outlook、iCloud 的 OAuth 登录，可使用本地凭证或 MailYou OAuth 代理
- 支持 AppImage、deb、rpm、zip 等 Linux 打包目标

## 技术栈

- Vue 3
- Vuetify
- Electron
- Rust
- Vite
- Pinia

## 环境要求

- Node.js
- npm
- Rust toolchain，包含 `cargo`

## 开发

安装依赖：

```bash
npm install
```

启动桌面开发模式：

```bash
npm run dev
```

启动 Web 目标：

```bash
npm run dev:web
```

## 构建

构建桌面应用：

```bash
npm run build
```

构建 Linux 包：

```bash
npm run dist:appimage
npm run dist:deb
npm run dist:rpm
```

构建 Windows 包：

```bash
npm run dist:win
```

## OAuth

MailYou 支持两种 OAuth 路径：

- 使用本地环境变量中的服务商凭证进行直连 OAuth
- 使用 MailYou OAuth 代理进行 OAuth

桌面应用当前内置以下默认值：

- 代理地址：`https://oauth2-proxy.iscccc.cc`
- 回调协议：`mailyou://oauth/callback`
- 代理 token：内置默认值，同时支持环境变量覆盖

如果某个服务商未配置直连 OAuth，MailYou 会回退到代理流程。

## 配置

详细配置文档见：

- [环境变量说明](docs/environment.md)

当前共享配置入口：

- [邮箱服务商预设](src/config/mailProviders.ts)
- [OAuth 默认配置](src/config/oauth.ts)
- [运行时默认配置](src/config/runtime.ts)

## 服务商预设

当前内置预设覆盖：

- 支持 OAuth 的域名：Gmail、Outlook、iCloud
- 使用密码的域名：QQ、Foxmail、163、126、Yeah

未命中预设的域名会回退到手动 IMAP / SMTP 配置。

## 项目结构

```text
src/
  config/      共享配置
  electron/    Electron 主进程与桌面集成
  rust/        Rust 邮件后端
  views/       Vue 页面
docs/          项目文档
```

## 给打包者的说明

- OAuth 代理默认可直接使用，桌面应用内置了公开的 desktop token。
- 打包者可以通过环境变量覆盖代理地址或代理 token。
- Linux 开发模式下默认不会注册 `mailyou://` 协议，除非显式开启。

