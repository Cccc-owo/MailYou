import type { MailyouBridge } from '../shared/mail/bridge'
import type { WindowControlsBridge } from '../shared/window/bridge'

declare global {
  interface Window {
    mailyou?: MailyouBridge
    windowControls?: WindowControlsBridge
  }
}

export {}
