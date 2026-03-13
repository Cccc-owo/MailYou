import { electronMailRepository } from '@/services/electronMailRepository'
import { webMailRepository } from '@/services/webMailRepository'
import type { MailRepository } from '@/shared/mail/mailRepository'

export const mailRepository: MailRepository =
  __MAILYOU_RUNTIME__ === 'web' ? webMailRepository : electronMailRepository
