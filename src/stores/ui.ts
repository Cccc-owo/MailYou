import { ref } from 'vue'
import { defineStore } from 'pinia'

export type AppearanceMode = 'light' | 'dark'
export type DensityMode = 'comfortable' | 'compact'

const APPEARANCE_KEY = 'mailstack.appearance'
const THEME_SEED_KEY = 'mailstack.themeSeed'
const DENSITY_KEY = 'mailstack.density'

export const useUiStore = defineStore('ui', () => {
  const appearance = ref<AppearanceMode>((localStorage.getItem(APPEARANCE_KEY) as AppearanceMode) || 'light')
  const themeSeed = ref(localStorage.getItem(THEME_SEED_KEY) || '#6750A4')
  const density = ref<DensityMode>((localStorage.getItem(DENSITY_KEY) as DensityMode) || 'comfortable')

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

  return {
    appearance,
    themeSeed,
    density,
    setAppearance,
    toggleAppearance,
    setThemeSeed,
    setDensity,
  }
})
