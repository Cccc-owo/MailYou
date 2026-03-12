import { ref } from 'vue'
import { defineStore } from 'pinia'
import { mailRepository } from '@/services/mail'
import type { DraftMessage, MailMessage } from '@/types/mail'

const createEmptyDraft = (): DraftMessage => ({
  id: `draft-${crypto.randomUUID()}`,
  accountId: '',
  to: '',
  cc: '',
  bcc: '',
  subject: '',
  body: '',
})

export const useComposerStore = defineStore('composer', () => {
  const isOpen = ref(false)
  const isSending = ref(false)
  const draft = ref<DraftMessage>(createEmptyDraft())

  const openNew = (accountId: string) => {
    draft.value = { ...createEmptyDraft(), accountId }
    isOpen.value = true
  }

  const openReply = (accountId: string, message: MailMessage) => {
    draft.value = {
      ...createEmptyDraft(),
      accountId,
      to: message.fromEmail,
      subject: message.subject.startsWith('Re:') ? message.subject : `Re: ${message.subject}`,
      body: `\n\n---\n${message.body.replace(/<[^>]+>/g, '')}`,
      inReplyToMessageId: message.id,
    }
    isOpen.value = true
  }

  const openForward = (accountId: string, message: MailMessage) => {
    draft.value = {
      ...createEmptyDraft(),
      accountId,
      subject: message.subject.startsWith('Fwd:') ? message.subject : `Fwd: ${message.subject}`,
      body: `\n\n--- Forwarded message ---\n${message.body.replace(/<[^>]+>/g, '')}`,
      forwardFromMessageId: message.id,
    }
    isOpen.value = true
  }

  const saveDraft = async () => {
    draft.value = await mailRepository.saveDraft(draft.value)
  }

  const sendDraft = async () => {
    isSending.value = true

    try {
      await mailRepository.sendMessage(draft.value)
      isOpen.value = false
      draft.value = createEmptyDraft()
    } finally {
      isSending.value = false
    }
  }

  const close = () => {
    isOpen.value = false
  }

  return {
    isOpen,
    isSending,
    draft,
    openNew,
    openReply,
    openForward,
    saveDraft,
    sendDraft,
    close,
  }
})
