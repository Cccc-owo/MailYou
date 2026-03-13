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
  attachments: [],
})

export const useComposerStore = defineStore('composer', () => {
  const isOpen = ref(false)
  const isSending = ref(false)
  const isSaving = ref(false)
  const error = ref<string | null>(null)
  const successMessage = ref<string | null>(null)
  const draft = ref<DraftMessage>(createEmptyDraft())

  const openNew = (accountId: string) => {
    error.value = null
    successMessage.value = null
    draft.value = { ...createEmptyDraft(), accountId }
    isOpen.value = true
  }

  const openReply = (accountId: string, message: MailMessage) => {
    error.value = null
    successMessage.value = null
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

  const openReplyAll = (accountId: string, message: MailMessage, selfEmail: string) => {
    error.value = null
    successMessage.value = null

    const self = selfEmail.toLowerCase()
    const allRecipients = [message.fromEmail, ...message.to, ...message.cc]
      .map((addr) => addr.trim())
      .filter((addr) => addr.length > 0 && addr.toLowerCase() !== self)
    const uniqueRecipients = [...new Set(allRecipients)]

    draft.value = {
      ...createEmptyDraft(),
      accountId,
      to: uniqueRecipients.join(', '),
      subject: message.subject.startsWith('Re:') ? message.subject : `Re: ${message.subject}`,
      body: `\n\n---\n${message.body.replace(/<[^>]+>/g, '')}`,
      inReplyToMessageId: message.id,
    }
    isOpen.value = true
  }

  const openForward = (accountId: string, message: MailMessage) => {
    error.value = null
    successMessage.value = null
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
    isSaving.value = true
    error.value = null
    successMessage.value = null

    try {
      draft.value = await mailRepository.saveDraft(draft.value)
      successMessage.value = 'Draft saved'
      return draft.value
    } catch (saveError) {
      error.value = saveError instanceof Error ? saveError.message : 'Unable to save draft'
      throw saveError
    } finally {
      isSaving.value = false
    }
  }

  const sendDraft = async () => {
    isSending.value = true
    error.value = null
    successMessage.value = null

    try {
      await mailRepository.sendMessage(draft.value)
      successMessage.value = 'Message sent'
      isOpen.value = false
      draft.value = createEmptyDraft()
    } catch (sendError) {
      error.value = sendError instanceof Error ? sendError.message : 'Unable to send message'
      throw sendError
    } finally {
      isSending.value = false
    }
  }

  const close = () => {
    isOpen.value = false
  }

  const clearFeedback = () => {
    error.value = null
    successMessage.value = null
  }

  return {
    isOpen,
    isSending,
    isSaving,
    error,
    successMessage,
    draft,
    openNew,
    openReply,
    openReplyAll,
    openForward,
    saveDraft,
    sendDraft,
    close,
    clearFeedback,
  }
})
