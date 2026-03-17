import { ref, watch } from 'vue'
import { defineStore } from 'pinia'
import { mailRepository } from '@/services/mail'
import { applyIdentitySignature as applyIdentitySignatureToBody } from '@/shared/mail/signature'
import { useAccountsStore } from '@/stores/accounts'
import type { DraftAttachment, DraftMessage, MailMessage } from '@/types/mail'

const LEGACY_DRAFT_STORAGE_KEY = 'mailyou:auto-saved-draft'
const DRAFT_STORAGE_KEY_PREFIX = 'mailyou:auto-saved-draft'
const DRAFT_INDEX_STORAGE_KEY = `${DRAFT_STORAGE_KEY_PREFIX}:index`
const AUTO_SAVE_DELAY = 2000

const IDB_NAME = 'mailyou'
const IDB_VERSION = 1
const IDB_STORE = 'draft-attachments'
const LEGACY_IDB_KEY = 'auto-saved'

type DraftStatus = 'local-only' | 'server-saved' | 'server-saved-with-local-changes'

interface DraftSnapshotEntry {
  draftId: string
  basedOnServerDraft: boolean
  updatedAt: string
}

interface DraftSnapshotIndex {
  lastStandaloneDraftId: string | null
  entries: Record<string, DraftSnapshotEntry>
}

const textStorageKey = (draftId: string) => `${DRAFT_STORAGE_KEY_PREFIX}:text:${draftId}`
const attachmentStorageKey = (draftId: string) => `${DRAFT_STORAGE_KEY_PREFIX}:attachments:${draftId}`

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

async function saveAttachmentsIDB(draftId: string, attachments: DraftAttachment[]): Promise<void> {
  const db = await openIDB()
  return new Promise((resolve, reject) => {
    const tx = db.transaction(IDB_STORE, 'readwrite')
    if (attachments.length > 0) {
      tx.objectStore(IDB_STORE).put(attachments, attachmentStorageKey(draftId))
    } else {
      tx.objectStore(IDB_STORE).delete(attachmentStorageKey(draftId))
    }
    tx.oncomplete = () => { db.close(); resolve() }
    tx.onerror = () => { db.close(); reject(tx.error) }
  })
}

async function loadAttachmentsIDB(draftId: string): Promise<DraftAttachment[]> {
  const db = await openIDB()
  return new Promise((resolve, reject) => {
    const tx = db.transaction(IDB_STORE, 'readonly')
    const req = tx.objectStore(IDB_STORE).get(attachmentStorageKey(draftId))
    req.onsuccess = () => {
      if (req.result) {
        db.close()
        resolve(req.result)
        return
      }
      const legacyReq = tx.objectStore(IDB_STORE).get(LEGACY_IDB_KEY)
      legacyReq.onsuccess = () => { db.close(); resolve(legacyReq.result ?? []) }
      legacyReq.onerror = () => { db.close(); reject(legacyReq.error) }
    }
    req.onerror = () => { db.close(); reject(req.error) }
  })
}

async function clearAttachmentsIDB(draftId: string): Promise<void> {
  const db = await openIDB()
  return new Promise((resolve, reject) => {
    const tx = db.transaction(IDB_STORE, 'readwrite')
    tx.objectStore(IDB_STORE).delete(attachmentStorageKey(draftId))
    tx.objectStore(IDB_STORE).delete(LEGACY_IDB_KEY)
    tx.oncomplete = () => { db.close(); resolve() }
    tx.onerror = () => { db.close(); reject(tx.error) }
  })
}

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
  persistenceState: 'local-only',
})

const isDraftEmpty = (d: Pick<DraftMessage, 'to' | 'subject' | 'body'>) => {
  const body = d.body.replace(/<[^>]*>/g, '').trim()
  return !d.to.trim() && !d.subject.trim() && !body
}

const emptyDraftSnapshotIndex = (): DraftSnapshotIndex => ({
  lastStandaloneDraftId: null,
  entries: {},
})

const loadSnapshotIndex = (): DraftSnapshotIndex => {
  try {
    const raw = localStorage.getItem(DRAFT_INDEX_STORAGE_KEY)
    if (!raw) {
      return emptyDraftSnapshotIndex()
    }
    const parsed = JSON.parse(raw) as Partial<DraftSnapshotIndex>
    return {
      lastStandaloneDraftId: parsed.lastStandaloneDraftId ?? null,
      entries: parsed.entries ?? {},
    }
  } catch {
    return emptyDraftSnapshotIndex()
  }
}

const saveSnapshotIndex = (index: DraftSnapshotIndex) => {
  try {
    localStorage.setItem(DRAFT_INDEX_STORAGE_KEY, JSON.stringify(index))
  } catch {
    // ignore quota errors
  }
}

const saveTextToLS = (draft: DraftMessage) => {
  try {
    const { attachments: _, ...rest } = draft
    localStorage.setItem(textStorageKey(draft.id), JSON.stringify(rest))
  } catch {
    // ignore quota errors
  }
}

const loadLegacyTextFromLS = (): Omit<DraftMessage, 'attachments'> | null => {
  try {
    const raw = localStorage.getItem(LEGACY_DRAFT_STORAGE_KEY)
    if (!raw) {
      return null
    }
    return JSON.parse(raw)
  } catch {
    return null
  }
}

const loadTextFromLS = (draftId: string): Omit<DraftMessage, 'attachments'> | null => {
  try {
    const raw = localStorage.getItem(textStorageKey(draftId))
    if (!raw) {
      return null
    }
    return JSON.parse(raw)
  } catch {
    return null
  }
}

const clearLS = (draftId: string) => {
  localStorage.removeItem(textStorageKey(draftId))
  localStorage.removeItem(LEGACY_DRAFT_STORAGE_KEY)
}

const normalizeDraftForLocal = (draft: DraftMessage, basedOnServerDraft: boolean): DraftMessage => ({
  ...draft,
  persistenceState: basedOnServerDraft ? 'server-saved-with-local-changes' : 'local-only',
  localAutosaveAt: new Date().toISOString(),
})

const saveDraftToLocal = (draft: DraftMessage, basedOnServerDraft: boolean) => {
  const normalized = normalizeDraftForLocal(draft, basedOnServerDraft)
  const index = loadSnapshotIndex()
  index.entries[normalized.id] = {
    draftId: normalized.id,
    basedOnServerDraft,
    updatedAt: normalized.localAutosaveAt ?? new Date().toISOString(),
  }
  if (!basedOnServerDraft) {
    index.lastStandaloneDraftId = normalized.id
  }
  saveSnapshotIndex(index)
  saveTextToLS(normalized)
  saveAttachmentsIDB(normalized.id, normalized.attachments).catch(() => {})
}

const clearLocalDraft = (draftId: string) => {
  const index = loadSnapshotIndex()
  delete index.entries[draftId]
  if (index.lastStandaloneDraftId === draftId) {
    index.lastStandaloneDraftId = null
  }
  saveSnapshotIndex(index)
  clearLS(draftId)
  clearAttachmentsIDB(draftId).catch(() => {})
}

const loadLocalDraftSnapshot = async (draftId: string): Promise<DraftMessage | null> => {
  const saved = loadTextFromLS(draftId)
  const legacySaved = loadLegacyTextFromLS()
  const resolved = saved ?? (legacySaved?.id === draftId ? legacySaved : null)
  if (!resolved) {
    return null
  }
  const attachments = await loadAttachmentsIDB(draftId).catch(() => [] as DraftAttachment[])
  return { ...resolved, attachments }
}

const getStandaloneRecoveryDraftId = () => {
  const index = loadSnapshotIndex()
  if (index.lastStandaloneDraftId && index.entries[index.lastStandaloneDraftId]) {
    return index.lastStandaloneDraftId
  }
  const legacyDraft = loadLegacyTextFromLS()
  if (legacyDraft?.id) {
    return legacyDraft.id
  }
  return null
}

const parseAddress = (value: string) => {
  const trimmed = value.trim().toLowerCase()
  const match = trimmed.match(/<([^>]+)>/)
  return (match?.[1] ?? trimmed).trim()
}

export const useComposerStore = defineStore('composer', () => {
  const accountsStore = useAccountsStore()
  const isOpen = ref(false)
  const isSending = ref(false)
  const isSaving = ref(false)
  const error = ref<string | null>(null)
  const successMessage = ref<string | null>(null)
  const draft = ref<DraftMessage>(createEmptyDraft())
  const draftStatus = ref<DraftStatus>('local-only')

  const showRecoveryDialog = ref(false)
  const pendingOpenAction = ref<(() => void) | null>(null)
  const pendingRecoveryDraftId = ref<string | null>(null)
  const recoveryMode = ref<'standalone' | 'existing'>('standalone')

  let autoSaveTimer: ReturnType<typeof setTimeout> | null = null
  let suppressAutoSave = false

  const setDraftState = (nextDraft: DraftMessage, status: DraftStatus) => {
    suppressAutoSave = true
    draft.value = {
      ...nextDraft,
      persistenceState: status,
      savedAt: status === 'server-saved' ? (nextDraft.savedAt ?? new Date().toISOString()) : nextDraft.savedAt,
      localAutosaveAt: status === 'server-saved' ? undefined : nextDraft.localAutosaveAt,
    }
    draftStatus.value = status
    queueMicrotask(() => {
      suppressAutoSave = false
    })
  }

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

  const openDraft = (nextDraft: DraftMessage, status: DraftStatus) => {
    error.value = null
    successMessage.value = null
    setDraftState(nextDraft, status)
    isOpen.value = true
  }

  const tryOpen = (openFn: () => void) => {
    const recoveryDraftId = getStandaloneRecoveryDraftId()
    const saved = recoveryDraftId ? loadTextFromLS(recoveryDraftId) : null
    if (saved && !isDraftEmpty(saved)) {
      pendingOpenAction.value = openFn
      pendingRecoveryDraftId.value = recoveryDraftId
      recoveryMode.value = 'standalone'
      showRecoveryDialog.value = true
      return
    }

    if (recoveryDraftId) {
      clearLocalDraft(recoveryDraftId)
    }
    openFn()
  }

  const recoverDraft = async () => {
    if (!pendingRecoveryDraftId.value) {
      return
    }

    const saved = await loadLocalDraftSnapshot(pendingRecoveryDraftId.value)
    if (saved) {
      error.value = null
      successMessage.value = null
      setDraftState(saved, saved.persistenceState ?? 'local-only')
      isOpen.value = true
    }

    pendingOpenAction.value = null
    pendingRecoveryDraftId.value = null
    showRecoveryDialog.value = false
  }

  const discardAndProceed = () => {
    if (pendingRecoveryDraftId.value) {
      clearLocalDraft(pendingRecoveryDraftId.value)
    }
    showRecoveryDialog.value = false
    if (pendingOpenAction.value) {
      pendingOpenAction.value()
      pendingOpenAction.value = null
    }
    pendingRecoveryDraftId.value = null
  }

  const cancelRecovery = () => {
    showRecoveryDialog.value = false
    pendingOpenAction.value = null
    pendingRecoveryDraftId.value = null
  }

  const openNew = (accountId: string) => {
    tryOpen(() => {
      openDraft(applyIdentitySignature({ ...createEmptyDraft(), accountId }, true), 'local-only')
    })
  }

  const openReply = (accountId: string, message: MailMessage) => {
    tryOpen(() => {
      openDraft(applyIdentitySignature({
        ...createEmptyDraft(),
        accountId,
        to: message.fromEmail,
        subject: message.subject.startsWith('Re:') ? message.subject : `Re: ${message.subject}`,
        body: `<br><br><blockquote>${message.body}</blockquote>`,
        inReplyToMessageId: message.id,
      }, true), 'local-only')
    })
  }

  const openReplyAll = (accountId: string, message: MailMessage, selfEmail: string) => {
    tryOpen(() => {
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

      openDraft(applyIdentitySignature({
        ...createEmptyDraft(),
        accountId,
        to: uniqueRecipients.join(', '),
        subject: message.subject.startsWith('Re:') ? message.subject : `Re: ${message.subject}`,
        body: `<br><br><blockquote>${message.body}</blockquote>`,
        inReplyToMessageId: message.id,
      }, true), 'local-only')
    })
  }

  const openForward = (accountId: string, message: MailMessage) => {
    tryOpen(() => {
      openDraft(applyIdentitySignature({
        ...createEmptyDraft(),
        accountId,
        subject: message.subject.startsWith('Fwd:') ? message.subject : `Fwd: ${message.subject}`,
        body: `<br><br><p>--- Forwarded message ---</p><blockquote>${message.body}</blockquote>`,
        forwardFromMessageId: message.id,
      }, true), 'local-only')
    })
  }

  const openExistingDraft = (nextDraft: DraftMessage) => {
    const saved = loadTextFromLS(nextDraft.id)
    if (saved && !isDraftEmpty(saved)) {
      pendingRecoveryDraftId.value = nextDraft.id
      pendingOpenAction.value = () => {
        openDraft({
          ...nextDraft,
          attachments: [...nextDraft.attachments],
          savedAt: nextDraft.savedAt ?? new Date().toISOString(),
        }, 'server-saved')
      }
      recoveryMode.value = 'existing'
      showRecoveryDialog.value = true
      return
    }

    openDraft({
      ...nextDraft,
      attachments: [...nextDraft.attachments],
      savedAt: nextDraft.savedAt ?? new Date().toISOString(),
    }, 'server-saved')
  }

  const saveDraft = async () => {
    isSaving.value = true
    error.value = null
    successMessage.value = null

    try {
      const savedAt = new Date().toISOString()
      const savedDraft = await mailRepository.saveDraft(draft.value)
      clearLocalDraft(draft.value.id)
      setDraftState({ ...savedDraft, savedAt }, 'server-saved')
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
      clearLocalDraft(draft.value.id)
      setDraftState(createEmptyDraft(), 'local-only')
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

  watch(
    draft,
    (newDraft) => {
      if (!isOpen.value || suppressAutoSave) return
      if (autoSaveTimer) clearTimeout(autoSaveTimer)
      autoSaveTimer = setTimeout(() => {
        if (!isDraftEmpty(newDraft)) {
          const basedOnServerDraft = draftStatus.value !== 'local-only'
          saveDraftToLocal(newDraft, basedOnServerDraft)
          if (draftStatus.value === 'server-saved') {
            draftStatus.value = 'server-saved-with-local-changes'
          }
        }
      }, AUTO_SAVE_DELAY)
    },
    { deep: true },
  )

  watch(isOpen, (open, wasOpen) => {
    if (wasOpen && !open) {
      if (autoSaveTimer) {
        clearTimeout(autoSaveTimer)
        autoSaveTimer = null
      }
      if (!isDraftEmpty(draft.value) && draftStatus.value !== 'server-saved') {
        saveDraftToLocal(draft.value, draftStatus.value !== 'local-only')
      } else {
        clearLocalDraft(draft.value.id)
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
    draftStatus,
    showRecoveryDialog,
    recoveryMode,
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
