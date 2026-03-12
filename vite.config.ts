import { fileURLToPath, URL } from 'node:url'
import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import electron from 'vite-plugin-electron/simple'
import vuetify from 'vite-plugin-vuetify'

const target = process.env.MAILSTACK_TARGET ?? 'electron'
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
            },
            preload: {
              input: 'src/electron/preload.ts',
            },
          }),
        ]
      : []),
  ],
  define: {
    __MAILSTACK_RUNTIME__: JSON.stringify(isWebTarget ? 'web' : 'electron'),
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
