import { createI18n } from 'vue-i18n'
import en from './locales/en.json'
import zh from './locales/zh.json'

const savedLocale = localStorage.getItem('mailstack.locale')
const defaultLocale = savedLocale || (navigator.language.startsWith('zh') ? 'zh' : 'en')

export const i18n = createI18n({
  legacy: false,
  locale: defaultLocale,
  fallbackLocale: 'en',
  messages: { en, zh },
})
