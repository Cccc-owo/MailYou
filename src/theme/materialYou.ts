export interface ThemePalette {
  background: string
  surface: string
  surfaceVariant: string
  primary: string
  primaryContainer: string
  secondary: string
  secondaryContainer: string
  accent: string
  error: string
  info: string
  success: string
  warning: string
  onBackground: string
  onSurface: string
  onPrimary: string
}

export interface ThemeDefinitionSet {
  light: ThemePalette
  dark: ThemePalette
}

const clamp = (value: number, min: number, max: number) => Math.min(Math.max(value, min), max)

const hexToRgb = (hex: string) => {
  const sanitized = hex.replace('#', '')
  const normalized = sanitized.length === 3
    ? sanitized
        .split('')
        .map((char) => `${char}${char}`)
        .join('')
    : sanitized

  const numeric = Number.parseInt(normalized, 16)

  return {
    r: (numeric >> 16) & 255,
    g: (numeric >> 8) & 255,
    b: numeric & 255,
  }
}

const rgbToHex = (r: number, g: number, b: number) =>
  `#${[r, g, b]
    .map((channel) => clamp(Math.round(channel), 0, 255).toString(16).padStart(2, '0'))
    .join('')}`

const mix = (base: string, target: string, amount: number) => {
  const a = hexToRgb(base)
  const b = hexToRgb(target)

  return rgbToHex(
    a.r + (b.r - a.r) * amount,
    a.g + (b.g - a.g) * amount,
    a.b + (b.b - a.b) * amount,
  )
}

export const createMaterialYouTheme = (seed: string): ThemeDefinitionSet => ({
  light: {
    background: mix(seed, '#ffffff', 0.94),
    surface: mix(seed, '#ffffff', 0.91),
    surfaceVariant: mix(seed, '#f1f3f4', 0.82),
    primary: mix(seed, '#000000', 0.08),
    primaryContainer: mix(seed, '#ffffff', 0.72),
    secondary: mix(seed, '#355c7d', 0.45),
    secondaryContainer: mix(seed, '#ffffff', 0.8),
    accent: mix(seed, '#7c4dff', 0.22),
    error: '#ba1a1a',
    info: '#245fa6',
    success: '#2e7d32',
    warning: '#a05a00',
    onBackground: '#1a1b1f',
    onSurface: '#1a1b1f',
    onPrimary: '#ffffff',
  },
  dark: {
    background: mix(seed, '#101114', 0.88),
    surface: mix(seed, '#181a1f', 0.84),
    surfaceVariant: mix(seed, '#272a31', 0.76),
    primary: mix(seed, '#ffffff', 0.18),
    primaryContainer: mix(seed, '#2d3142', 0.52),
    secondary: mix(seed, '#c6d6f5', 0.48),
    secondaryContainer: mix(seed, '#253045', 0.4),
    accent: mix(seed, '#b388ff', 0.3),
    error: '#f4a9a0',
    info: '#9dc4f5',
    success: '#8ed091',
    warning: '#edb76a',
    onBackground: '#eceef4',
    onSurface: '#eceef4',
    onPrimary: '#101114',
  },
})
