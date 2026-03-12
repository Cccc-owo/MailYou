import type { MailRepository } from '../../shared/mail/mailRepository'
import { rustMailBackend } from './rust/mailBackend'

export type MailBackend = MailRepository

export const mailBackend: MailBackend = rustMailBackend
