import { computed, watch } from 'vue'
import { useTheme } from 'vuetify'
import { storeToRefs } from 'pinia'
import { useUiStore } from '@/stores/ui'
import { createMaterialYouTheme } from '@/theme/materialYou'

export const useThemeController = () => {
  const theme = useTheme()
  const uiStore = useUiStore()
  const { appearance, themeSeed } = storeToRefs(uiStore)

  const applyTheme = () => {
    const generated = createMaterialYouTheme(themeSeed.value)
    const light = theme.themes.value['mailyou-light']
    const dark = theme.themes.value['mailyou-dark']

    light.colors = {
      ...light.colors,
      background: generated.light.background,
      surface: generated.light.surface,
      'surface-variant': generated.light.surfaceVariant,
      primary: generated.light.primary,
      'primary-container': generated.light.primaryContainer,
      secondary: generated.light.secondary,
      'secondary-container': generated.light.secondaryContainer,
      accent: generated.light.accent,
      error: generated.light.error,
      info: generated.light.info,
      success: generated.light.success,
      warning: generated.light.warning,
      'on-background': generated.light.onBackground,
      'on-surface': generated.light.onSurface,
      'on-primary': generated.light.onPrimary,
    }

    dark.colors = {
      ...dark.colors,
      background: generated.dark.background,
      surface: generated.dark.surface,
      'surface-variant': generated.dark.surfaceVariant,
      primary: generated.dark.primary,
      'primary-container': generated.dark.primaryContainer,
      secondary: generated.dark.secondary,
      'secondary-container': generated.dark.secondaryContainer,
      accent: generated.dark.accent,
      error: generated.dark.error,
      info: generated.dark.info,
      success: generated.dark.success,
      warning: generated.dark.warning,
      'on-background': generated.dark.onBackground,
      'on-surface': generated.dark.onSurface,
      'on-primary': generated.dark.onPrimary,
    }

    theme.global.name.value = appearance.value === 'dark' ? 'mailyou-dark' : 'mailyou-light'
  }

  watch([appearance, themeSeed], applyTheme, { immediate: true })

  return {
    currentThemeName: computed(() => theme.global.name.value),
    applyTheme,
  }
}
