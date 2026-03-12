import type { MailstackBridge } from '../shared/mail/bridge'
import type { WindowControlsBridge } from '../shared/window/bridge'

declare global {
  interface Window {
    mailstack?: MailstackBridge
    windowControls?: WindowControlsBridge
  }
}

export {}
