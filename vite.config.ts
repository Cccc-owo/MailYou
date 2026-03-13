import { fileURLToPath, URL } from 'node:url'
import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import electron from 'vite-plugin-electron/simple'
import vuetify from 'vite-plugin-vuetify'

const target = process.env.MAILYOU_TARGET ?? 'electron'
const isDesktopTarget = target === 'electron'
const isWebTarget = target === 'web'

export default defineConfig({
  plugins: [
    vue(),
    vuetify({ autoImport: true }),
    ...(isDesktopTarget
      ? [
          electron({
            main: {
              entry: 'src/electron/main.ts',
              vite: { build: { outDir: 'dist/electron' } },
            },
            preload: {
              input: 'src/electron/preload.ts',
              vite: { build: { outDir: 'dist/electron' } },
            },
          }),
        ]
      : []),
  ],
  define: {
    __MAILYOU_RUNTIME__: JSON.stringify(isWebTarget ? 'web' : 'electron'),
  },
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
    },
  },
  server: {
    port: 1420,
    strictPort: true,
  },
  clearScreen: false,
})
