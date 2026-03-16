import { defineConfig } from 'vitepress'

const ogImage = '/logo.svg'
const githubLink = 'https://github.com/Cccc-owo/MailYou'

export default defineConfig({
  base: '/',
  title: 'MailYou',
  description: 'A desktop-first mail client built with Vue, Electron, and Rust.',
  lastUpdated: true,
  head: [
    ['link', { rel: 'icon', href: '/logo.svg', type: 'image/svg+xml' }],
    ['meta', { property: 'og:type', content: 'website' }],
    ['meta', { property: 'og:image', content: ogImage }],
    ['meta', { name: 'theme-color', content: '#1A73E8' }],
  ],
  themeConfig: {
    logo: '/logo.svg',
    nav: [
      { text: 'Guide', link: '/guide/' },
      { text: 'Download', link: '/release/' },
      { text: 'Legal', link: '/legal/privacy-policy' },
      { text: 'GitHub', link: githubLink },
      { text: '中文', link: '/zh/' },
    ],
    socialLinks: [{ icon: 'github', link: githubLink }],
    footer: {
      message: 'MailYou focuses on local-first desktop email.',
      copyright: 'Copyright © 2026 MailYou',
    },
    search: {
      provider: 'local',
    },
  },
  locales: {
    root: {
      label: 'English',
      lang: 'en-US',
      themeConfig: {
        nav: [
          { text: 'Guide', link: '/guide/' },
          { text: 'Download', link: '/release/' },
          { text: 'Legal', link: '/legal/privacy-policy' },
          { text: 'GitHub', link: githubLink },
          { text: '中文', link: '/zh/' },
        ],
        sidebar: [
          {
            text: 'Overview',
            items: [
              { text: 'Introduction', link: '/guide/' },
              { text: 'Download', link: '/release/' },
            ],
          },
          {
            text: 'Legal',
            items: [
              { text: 'Privacy Policy', link: '/legal/privacy-policy' },
              { text: 'Terms of Service', link: '/legal/terms-of-service' },
            ],
          },
        ],
        docFooter: {
          prev: 'Previous page',
          next: 'Next page',
        },
        outline: {
          label: 'On this page',
        },
      },
    },
    zh: {
      label: '中文',
      lang: 'zh-CN',
      title: 'MailYou',
      description: '使用 Vue、Electron 与 Rust 构建的桌面优先邮件客户端。',
      themeConfig: {
        nav: [
          { text: '简介', link: '/zh/guide/' },
          { text: '下载', link: '/zh/release/' },
          { text: '法律', link: '/zh/legal/privacy-policy' },
          { text: 'GitHub', link: githubLink },
          { text: 'English', link: '/' },
        ],
        sidebar: [
          {
            text: '概览',
            items: [
              { text: '项目介绍', link: '/zh/guide/' },
              { text: '下载', link: '/zh/release/' },
            ],
          },
          {
            text: '法律',
            items: [
              { text: '隐私政策', link: '/zh/legal/privacy-policy' },
              { text: '服务条款', link: '/zh/legal/terms-of-service' },
            ],
          },
        ],
        footer: {
          message: 'MailYou 专注于本地优先的桌面邮件体验。',
          copyright: 'Copyright © 2026 MailYou',
        },
        docFooter: {
          prev: '上一页',
          next: '下一页',
        },
        outline: {
          label: '本页内容',
        },
      },
    },
  },
})
