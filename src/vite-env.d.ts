/// <reference types="vite/client" />
/// <reference path="./electron/types.d.ts" />

declare const __MAILYOU_RUNTIME__: 'electron' | 'web'

declare module '*.vue' {
  import type { DefineComponent } from 'vue'
  const component: DefineComponent<Record<string, never>, Record<string, never>, any>
  export default component
}
