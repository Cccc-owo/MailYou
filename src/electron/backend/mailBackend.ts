import type { MailRepository } from '../../shared/mail/mailRepository'
import { rustMailBackend } from './rust/mailBackend'

export interface MailBackend extends MailRepository {
  getAccountUnreadSnapshot(accountId: string): Promise<{
    accountId: string
    unreadMessages: Array<{
      id: string
      subject: string
      from: string
    }>
    updatedAt: string
  }>
}

export const mailBackend: MailBackend = rustMailBackend
