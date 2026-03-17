import { ref } from 'vue'
import { defineStore } from 'pinia'
import { i18n } from '@/i18n'
import type { CloseBehaviorPreference } from '@/shared/window/bridge'

export type AppearanceMode = 'light' | 'dark'
export type LocaleMode = 'en' | 'zh'
export type ImageLoadPolicy = 'noRemote' | 'noHttp' | 'all'

const APPEARANCE_KEY = 'mailyou.appearance'
const THEME_SEED_KEY = 'mailyou.themeSeed'
const SYNC_INTERVAL_KEY = 'mailyou.syncIntervalMinutes'
const FETCH_LIMIT_KEY = 'mailyou.fetchLimit'
const LOCALE_KEY = 'mailyou.locale'
const IMAGE_LOAD_POLICY_KEY = 'mailyou.imageLoadPolicy'
const CLOSE_BEHAVIOR_KEY = 'mailyou.closeBehavior'

const getDefaultLocale = (): LocaleMode => {
  const saved = localStorage.getItem(LOCALE_KEY) as LocaleMode | null
  if (saved) return saved
  return navigator.language.startsWith('zh') ? 'zh' : 'en'
}

export const useUiStore = defineStore('ui', () => {
  const appearance = ref<AppearanceMode>((localStorage.getItem(APPEARANCE_KEY) as AppearanceMode) || 'light')
  const themeSeed = ref(localStorage.getItem(THEME_SEED_KEY) || '#6750A4')
  const syncIntervalMinutes = ref(Number(localStorage.getItem(SYNC_INTERVAL_KEY)) || 5)
  const fetchLimit = ref(Number(localStorage.getItem(FETCH_LIMIT_KEY)) || 50)
  const locale = ref<LocaleMode>(getDefaultLocale())
  const imageLoadPolicy = ref<ImageLoadPolicy>((localStorage.getItem(IMAGE_LOAD_POLICY_KEY) as ImageLoadPolicy) || 'noRemote')
  const closeBehavior = ref<CloseBehaviorPreference>((localStorage.getItem(CLOSE_BEHAVIOR_KEY) as CloseBehaviorPreference) || 'ask')

  const setAppearance = (value: AppearanceMode) => {
    appearance.value = value
    localStorage.setItem(APPEARANCE_KEY, appearance.value)
  }

  const toggleAppearance = () => {
    setAppearance(appearance.value === 'light' ? 'dark' : 'light')
  }

  const setThemeSeed = (value: string) => {
    themeSeed.value = value
    localStorage.setItem(THEME_SEED_KEY, value)
  }

  const setSyncIntervalMinutes = (value: number) => {
    syncIntervalMinutes.value = value
    localStorage.setItem(SYNC_INTERVAL_KEY, String(value))
  }

  const setFetchLimit = (value: number) => {
    fetchLimit.value = value
    localStorage.setItem(FETCH_LIMIT_KEY, String(value))
  }

  const setLocale = (value: LocaleMode) => {
    locale.value = value
    localStorage.setItem(LOCALE_KEY, value)
    i18n.global.locale.value = value
  }

  const setImageLoadPolicy = (value: ImageLoadPolicy) => {
    imageLoadPolicy.value = value
    localStorage.setItem(IMAGE_LOAD_POLICY_KEY, value)
  }

  const setCloseBehavior = (value: CloseBehaviorPreference) => {
    closeBehavior.value = value
    localStorage.setItem(CLOSE_BEHAVIOR_KEY, value)
  }

  return {
    appearance,
    themeSeed,
    syncIntervalMinutes,
    fetchLimit,
    locale,
    imageLoadPolicy,
    closeBehavior,
    setAppearance,
    toggleAppearance,
    setThemeSeed,
    setSyncIntervalMinutes,
    setFetchLimit,
    setLocale,
    setImageLoadPolicy,
    setCloseBehavior,
  }
})
