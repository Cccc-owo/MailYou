import type { MailRepository } from '@/shared/mail/mailRepository'

export interface MailyouBridge extends MailRepository {
  onBackgroundSync(callback: (accountId: string) => void): () => void
}
