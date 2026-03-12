import '@mdi/font/css/materialdesignicons.css'
import 'vuetify/styles'
import { createVuetify, type ThemeDefinition } from 'vuetify'
import { aliases, mdi } from 'vuetify/iconsets/mdi'
import * as components from 'vuetify/components'
import * as directives from 'vuetify/directives'
import { createMaterialYouTheme } from '@/theme/materialYou'

const defaultSeed = '#6750A4'
const generated = createMaterialYouTheme(defaultSeed)

const lightTheme: ThemeDefinition = {
  dark: false,
  colors: {
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
  },
}

const darkTheme: ThemeDefinition = {
  dark: true,
  colors: {
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
  },
}

export const vuetify = createVuetify({
  components,
  directives,
  icons: {
    defaultSet: 'mdi',
    aliases,
    sets: { mdi },
  },
  defaults: {
    VBtn: {
      rounded: 'pill',
      variant: 'tonal',
    },
    VCard: {
      rounded: 'xl',
    },
    VTextField: {
      variant: 'solo-filled',
      density: 'comfortable',
    },
  },
  theme: {
    defaultTheme: 'mailstack-light',
    themes: {
      'mailstack-light': lightTheme,
      'mailstack-dark': darkTheme,
    },
  },
})
