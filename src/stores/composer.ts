import { ref, watch } from 'vue'
import { defineStore } from 'pinia'
import { mailRepository } from '@/services/mail'
import { applyIdentitySignature as applyIdentitySignatureToBody } from '@/shared/mail/signature'
import { useAccountsStore } from '@/stores/accounts'
import type { DraftAttachment, DraftMessage, MailMessage } from '@/types/mail'

const DRAFT_STORAGE_KEY = 'mailyou:auto-saved-draft'
const AUTO_SAVE_DELAY = 2000

// ---------------------------------------------------------------------------
// IndexedDB helpers — stores attachments (base64 can be huge, localStorage
// would blow up). The text fields remain in localStorage for fast sync reads.
// ---------------------------------------------------------------------------

const IDB_NAME = 'mailyou'
const IDB_VERSION = 1
const IDB_STORE = 'draft-attachments'
const IDB_KEY = 'auto-saved'

function openIDB(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const req = indexedDB.open(IDB_NAME, IDB_VERSION)
    req.onupgradeneeded = () => {
      const db = req.result
      if (!db.objectStoreNames.contains(IDB_STORE)) {
        db.createObjectStore(IDB_STORE)
      }
    }
    req.onsuccess = () => resolve(req.result)
    req.onerror = () => reject(req.error)
  })
}

async function saveAttachmentsIDB(attachments: DraftAttachment[]): Promise<void> {
  const db = await openIDB()
  return new Promise((resolve, reject) => {
    const tx = db.transaction(IDB_STORE, 'readwrite')
    if (attachments.length > 0) {
      tx.objectStore(IDB_STORE).put(attachments, IDB_KEY)
    } else {
      tx.objectStore(IDB_STORE).delete(IDB_KEY)
    }
    tx.oncomplete = () => { db.close(); resolve() }
    tx.onerror = () => { db.close(); reject(tx.error) }
  })
}

async function loadAttachmentsIDB(): Promise<DraftAttachment[]> {
  const db = await openIDB()
  return new Promise((resolve, reject) => {
    const tx = db.transaction(IDB_STORE, 'readonly')
    const req = tx.objectStore(IDB_STORE).get(IDB_KEY)
    req.onsuccess = () => { db.close(); resolve(req.result ?? []) }
    req.onerror = () => { db.close(); reject(req.error) }
  })
}

async function clearAttachmentsIDB(): Promise<void> {
  const db = await openIDB()
  return new Promise((resolve, reject) => {
    const tx = db.transaction(IDB_STORE, 'readwrite')
    tx.objectStore(IDB_STORE).delete(IDB_KEY)
    tx.oncomplete = () => { db.close(); resolve() }
    tx.onerror = () => { db.close(); reject(tx.error) }
  })
}

// ---------------------------------------------------------------------------
// localStorage helpers — text fields only (fast, synchronous)
// ---------------------------------------------------------------------------

const createEmptyDraft = (): DraftMessage => ({
  id: `draft-${crypto.randomUUID()}`,
  accountId: '',
  selectedIdentityId: undefined,
  to: '',
  cc: '',
  bcc: '',
  subject: '',
  body: '',
  attachments: [],
})

const isDraftEmpty = (d: Pick<DraftMessage, 'to' | 'subject' | 'body'>) => {
  const body = d.body.replace(/<[^>]*>/g, '').trim()
  return !d.to.trim() && !d.subject.trim() && !body
}

const saveTextToLS = (d: DraftMessage) => {
  try {
    const { attachments: _, ...rest } = d
    localStorage.setItem(DRAFT_STORAGE_KEY, JSON.stringify(rest))
  } catch {
    // quota exceeded — silently ignore
  }
}

const loadTextFromLS = (): Omit<DraftMessage, 'attachments'> | null => {
  try {
    const raw = localStorage.getItem(DRAFT_STORAGE_KEY)
    if (!raw) return null
    return JSON.parse(raw)
  } catch {
    return null
  }
}

const clearLS = () => {
  localStorage.removeItem(DRAFT_STORAGE_KEY)
}

const parseAddress = (value: string) => {
  const trimmed = value.trim().toLowerCase()
  const match = trimmed.match(/<([^>]+)>/)
  return (match?.[1] ?? trimmed).trim()
}

// Combined save / clear (fire-and-forget for IndexedDB)
const saveDraftToLocal = (d: DraftMessage) => {
  saveTextToLS(d)
  saveAttachmentsIDB(d.attachments).catch(() => {})
}

const clearLocalDraft = () => {
  clearLS()
  clearAttachmentsIDB().catch(() => {})
}

// ---------------------------------------------------------------------------
// Store
// ---------------------------------------------------------------------------

export const useComposerStore = defineStore('composer', () => {
  const accountsStore = useAccountsStore()
  const isOpen = ref(false)
  const isSending = ref(false)
  const isSaving = ref(false)
  const error = ref<string | null>(null)
  const successMessage = ref<string | null>(null)
  const draft = ref<DraftMessage>(createEmptyDraft())

  // Recovery dialog state
  const showRecoveryDialog = ref(false)
  const pendingOpenAction = ref<(() => void) | null>(null)

  let autoSaveTimer: ReturnType<typeof setTimeout> | null = null

  const applyIdentitySignature = (nextDraft: DraftMessage, force = false) => {
    const account = accountsStore.accounts.find((account) => account.id === nextDraft.accountId)
    const identity = account?.identities.find((candidate) =>
      nextDraft.selectedIdentityId
        ? candidate.id === nextDraft.selectedIdentityId
        : candidate.isDefault,
    ) ?? account?.identities[0]

    if (!identity?.signature) {
      return nextDraft
    }

    return {
      ...nextDraft,
      selectedIdentityId: identity.id,
      body: applyIdentitySignatureToBody(nextDraft.body || '', identity.id, identity.signature, force),
    }
  }

  // -- Auto-save helpers ------------------------------------------------

  const tryOpen = (openFn: () => void) => {
    const saved = loadTextFromLS()
    if (saved && !isDraftEmpty(saved)) {
      pendingOpenAction.value = openFn
      showRecoveryDialog.value = true
    } else {
      clearLocalDraft()
      openFn()
    }
  }

  const recoverDraft = async () => {
    const saved = loadTextFromLS()
    if (saved) {
      error.value = null
      successMessage.value = null
      const attachments = await loadAttachmentsIDB().catch(() => [] as DraftAttachment[])
      draft.value = { ...saved, attachments }
      isOpen.value = true
    }
    pendingOpenAction.value = null
    showRecoveryDialog.value = false
  }

  const discardAndProceed = () => {
    clearLocalDraft()
    showRecoveryDialog.value = false
    if (pendingOpenAction.value) {
      pendingOpenAction.value()
      pendingOpenAction.value = null
    }
  }

  const cancelRecovery = () => {
    showRecoveryDialog.value = false
    pendingOpenAction.value = null
  }

  // -- Open actions -----------------------------------------------------

  const openNew = (accountId: string) => {
    tryOpen(() => {
      error.value = null
      successMessage.value = null
      draft.value = applyIdentitySignature({ ...createEmptyDraft(), accountId }, true)
      isOpen.value = true
    })
  }

  const openReply = (accountId: string, message: MailMessage) => {
    tryOpen(() => {
      error.value = null
      successMessage.value = null
      draft.value = applyIdentitySignature({
        ...createEmptyDraft(),
        accountId,
        to: message.fromEmail,
        subject: message.subject.startsWith('Re:') ? message.subject : `Re: ${message.subject}`,
        body: `<br><br><blockquote>${message.body}</blockquote>`,
        inReplyToMessageId: message.id,
      }, true)
      isOpen.value = true
    })
  }

  const openReplyAll = (accountId: string, message: MailMessage, selfEmail: string) => {
    tryOpen(() => {
      error.value = null
      successMessage.value = null

      const account = accountsStore.accounts.find((candidate) => candidate.id === accountId)
      const selfAddresses = new Set(
        [
          selfEmail,
          account?.email,
          ...(account?.identities ?? []).map((identity) => identity.email),
        ]
          .filter(Boolean)
          .map((value) => parseAddress(String(value))),
      )
      const allRecipients = [message.fromEmail, ...message.to, ...message.cc]
        .map((addr) => addr.trim())
        .filter((addr) => addr.length > 0 && !selfAddresses.has(parseAddress(addr)))
      const uniqueRecipients = [...new Set(allRecipients)]

      draft.value = applyIdentitySignature({
        ...createEmptyDraft(),
        accountId,
        to: uniqueRecipients.join(', '),
        subject: message.subject.startsWith('Re:') ? message.subject : `Re: ${message.subject}`,
        body: `<br><br><blockquote>${message.body}</blockquote>`,
        inReplyToMessageId: message.id,
      }, true)
      isOpen.value = true
    })
  }

  const openForward = (accountId: string, message: MailMessage) => {
    tryOpen(() => {
      error.value = null
      successMessage.value = null
      draft.value = applyIdentitySignature({
        ...createEmptyDraft(),
        accountId,
        subject: message.subject.startsWith('Fwd:') ? message.subject : `Fwd: ${message.subject}`,
        body: `<br><br><p>--- Forwarded message ---</p><blockquote>${message.body}</blockquote>`,
        forwardFromMessageId: message.id,
      }, true)
      isOpen.value = true
    })
  }

  const openExistingDraft = (nextDraft: DraftMessage) => {
    error.value = null
    successMessage.value = null
    const saved = loadTextFromLS()
    if (saved && saved.id === nextDraft.id && !isDraftEmpty(saved)) {
      void loadAttachmentsIDB()
        .then((attachments) => {
          draft.value = { ...saved, attachments }
          isOpen.value = true
        })
        .catch(() => {
          draft.value = { ...saved, attachments: [] }
          isOpen.value = true
        })
      return
    }

    draft.value = { ...nextDraft, attachments: [...nextDraft.attachments] }
    isOpen.value = true
  }

  // -- Draft save / send ------------------------------------------------

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
      draft.value = createEmptyDraft()
      clearLocalDraft()
      isOpen.value = false
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

  // -- Watchers ---------------------------------------------------------

  // Debounced auto-save while composer is open
  watch(
    draft,
    (newDraft) => {
      if (!isOpen.value) return
      if (autoSaveTimer) clearTimeout(autoSaveTimer)
      autoSaveTimer = setTimeout(() => {
        if (!isDraftEmpty(newDraft)) {
          saveDraftToLocal(newDraft)
        }
      }, AUTO_SAVE_DELAY)
    },
    { deep: true },
  )

  // Flush or clear on close
  watch(isOpen, (open, wasOpen) => {
    if (wasOpen && !open) {
      if (autoSaveTimer) {
        clearTimeout(autoSaveTimer)
        autoSaveTimer = null
      }
      if (!isDraftEmpty(draft.value)) {
        saveDraftToLocal(draft.value)
      } else {
        clearLocalDraft()
      }
    }
  })

  return {
    isOpen,
    isSending,
    isSaving,
    error,
    successMessage,
    draft,
    showRecoveryDialog,
    openNew,
    openReply,
    openReplyAll,
    openForward,
    openExistingDraft,
    recoverDraft,
    discardAndProceed,
    cancelRecovery,
    saveDraft,
    sendDraft,
    close,
    clearFeedback,
  }
})
