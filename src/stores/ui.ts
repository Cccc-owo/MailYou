import { ref } from 'vue'
import { defineStore } from 'pinia'

export type AppearanceMode = 'light' | 'dark'
export type DensityMode = 'comfortable' | 'compact'

const APPEARANCE_KEY = 'mailstack.appearance'
const THEME_SEED_KEY = 'mailstack.themeSeed'
const DENSITY_KEY = 'mailstack.density'
const SYNC_INTERVAL_KEY = 'mailstack.syncIntervalMinutes'
const FETCH_LIMIT_KEY = 'mailstack.fetchLimit'

export const useUiStore = defineStore('ui', () => {
  const appearance = ref<AppearanceMode>((localStorage.getItem(APPEARANCE_KEY) as AppearanceMode) || 'light')
  const themeSeed = ref(localStorage.getItem(THEME_SEED_KEY) || '#6750A4')
  const density = ref<DensityMode>((localStorage.getItem(DENSITY_KEY) as DensityMode) || 'comfortable')
  const syncIntervalMinutes = ref(Number(localStorage.getItem(SYNC_INTERVAL_KEY)) || 5)
  const fetchLimit = ref(Number(localStorage.getItem(FETCH_LIMIT_KEY)) || 50)

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

  const setDensity = (value: DensityMode) => {
    density.value = value
    localStorage.setItem(DENSITY_KEY, value)
  }

  const setSyncIntervalMinutes = (value: number) => {
    syncIntervalMinutes.value = value
    localStorage.setItem(SYNC_INTERVAL_KEY, String(value))
  }

  const setFetchLimit = (value: number) => {
    fetchLimit.value = value
    localStorage.setItem(FETCH_LIMIT_KEY, String(value))
  }

  return {
    appearance,
    themeSeed,
    density,
    syncIntervalMinutes,
    fetchLimit,
    setAppearance,
    toggleAppearance,
    setThemeSeed,
    setDensity,
    setSyncIntervalMinutes,
    setFetchLimit,
  }
})
