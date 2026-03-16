# MailYou

MailYou 是一个本地优先的桌面邮箱客户端，适合希望邮件体验更安静、更贴近桌面的人。

## 为桌面工作流而做

项目使用 Vue 构建界面，Electron 负责桌面集成，Rust 负责邮件协议与本地存储。目标很直接：让邮箱体验更舒适，同时把数据尽量留在设备本地。

## 添加邮箱

MailYou 的添加流程尽量保持简短：

- 从邮箱地址开始
- 命中已知域名时自动套用预设
- 仅在服务商支持时显示 OAuth
- 仅在没有预设或开启高级模式时显示服务器细节

## 认证方式

MailYou 支持两种 OAuth 路径：

- 使用本地 `client_id` 与 `client_secret` 直连服务商
- 使用内置默认地址和桌面 token 的 MailYou OAuth 代理

如果服务商不支持 OAuth，则回退为标准密码登录。

## 隐私方向

MailYou 以本地桌面存储为基础，不把自己定位为托管邮箱或云同步服务。

法律文档已发布在本站：

- [隐私政策](/zh/legal/privacy-policy)
- [服务条款](/zh/legal/terms-of-service)
