import { computed, ref } from 'vue'
import { mailRepository } from '@/services/mail'
import type { MailLabel, MailMessage } from '@/types/mail'

interface UseMailLabelDialogOptions {
  t: (key: string, params?: Record<string, unknown>) => string
  currentAccountId: () => string | null
  findMessage: (messageId: string) => MailMessage | undefined
  fetchAccountLabels: (accountId: string, options?: { force?: boolean }) => Promise<MailLabel[]>
  refreshMailbox: (options?: { reloadLabels?: boolean }) => Promise<void>
}

export const useMailLabelDialog = ({
  t,
  currentAccountId,
  findMessage,
  fetchAccountLabels,
  refreshMailbox,
}: UseMailLabelDialogOptions) => {
  const labelDialogOpen = ref(false)
  const labelDialogMessageIds = ref<string[]>([])
  const labelDialogBusy = ref(false)
  const labelDialogError = ref<string | null>(null)
  const labelDraftName = ref('')
  const labelRenameSource = ref<string | null>(null)
  const labelDialogLabels = ref<MailLabel[]>([])

  const labelDialogMessages = computed(() =>
    labelDialogMessageIds.value
      .map((messageId) => findMessage(messageId))
      .filter((message): message is NonNullable<typeof message> => Boolean(message)),
  )

  const labelDialogSummary = computed(() => {
    if (labelDialogMessages.value.length === 1) {
      return labelDialogMessages.value[0].subject || t('labels.manageHint')
    }
    if (labelDialogMessages.value.length > 1) {
      return t('labels.selectedMessages', { count: labelDialogMessages.value.length })
    }
    return t('labels.manageHint')
  })

  const resetLabelDialogState = () => {
    labelDialogBusy.value = false
    labelDialogError.value = null
    labelDraftName.value = ''
    labelRenameSource.value = null
    labelDialogLabels.value = []
  }

  const loadAccountLabels = async () => {
    const accountId = currentAccountId()
    if (!accountId) {
      labelDialogLabels.value = []
      return
    }

    labelDialogLabels.value = await fetchAccountLabels(accountId, { force: true })
  }

  const openLabelDialogForMessages = async (messageIds: string[]) => {
    if (!currentAccountId()) return

    labelDialogMessageIds.value = messageIds
    resetLabelDialogState()
    labelDialogOpen.value = true

    try {
      await loadAccountLabels()
    } catch (error) {
      labelDialogError.value = error instanceof Error ? error.message : t('labels.loadFailed')
    }
  }

  const openLabelDialog = async (messageId: string) => {
    await openLabelDialogForMessages([messageId])
  }

  const closeLabelDialog = () => {
    labelDialogOpen.value = false
    labelDialogMessageIds.value = []
    resetLabelDialogState()
  }

  const isLabelApplied = (label: string) =>
    labelDialogMessages.value.length > 0
      && labelDialogMessages.value.every((message) =>
        message.labels.some((item: string) => item.toLowerCase() === label.toLowerCase()),
      )

  const toggleMessageLabel = async (label: string) => {
    const accountId = currentAccountId()
    if (!accountId || labelDialogMessageIds.value.length === 0) return

    labelDialogBusy.value = true
    labelDialogError.value = null

    try {
      const applyToAll = !isLabelApplied(label)
      for (const messageId of labelDialogMessageIds.value) {
        if (applyToAll) {
          await mailRepository.addLabel(accountId, messageId, label)
        } else {
          await mailRepository.removeLabel(accountId, messageId, label)
        }
      }
      await refreshMailbox({ reloadLabels: true })
      await loadAccountLabels()
    } catch (error) {
      labelDialogError.value = error instanceof Error ? error.message : t('labels.updateFailed')
    } finally {
      labelDialogBusy.value = false
    }
  }

  const startRenameLabel = (label: string) => {
    labelRenameSource.value = label
    labelDraftName.value = label
    labelDialogError.value = null
  }

  const cancelLabelRename = () => {
    labelRenameSource.value = null
    labelDraftName.value = ''
    labelDialogError.value = null
  }

  const submitLabelDraft = async () => {
    const accountId = currentAccountId()
    const nextLabelName = labelDraftName.value.trim()
    if (!accountId || labelDialogMessageIds.value.length === 0 || !nextLabelName) return

    labelDialogBusy.value = true
    labelDialogError.value = null

    try {
      if (labelRenameSource.value) {
        labelDialogLabels.value = await mailRepository.renameLabel(
          accountId,
          labelRenameSource.value,
          nextLabelName,
        )
        await refreshMailbox({ reloadLabels: true })
        cancelLabelRename()
        return
      }

      for (const messageId of labelDialogMessageIds.value) {
        await mailRepository.addLabel(accountId, messageId, nextLabelName)
      }
      labelDraftName.value = ''
      await refreshMailbox({ reloadLabels: true })
      await loadAccountLabels()
    } catch (error) {
      labelDialogError.value = error instanceof Error ? error.message : t('labels.updateFailed')
    } finally {
      labelDialogBusy.value = false
    }
  }

  const deleteAccountLabel = async (label: string) => {
    const accountId = currentAccountId()
    if (!accountId) return
    if (!window.confirm(t('labels.deleteConfirm', { label }))) return

    labelDialogBusy.value = true
    labelDialogError.value = null

    try {
      labelDialogLabels.value = await mailRepository.deleteLabel(accountId, label)
      await refreshMailbox({ reloadLabels: true })
      if (labelRenameSource.value?.toLowerCase() === label.toLowerCase()) {
        cancelLabelRename()
      }
    } catch (error) {
      labelDialogError.value = error instanceof Error ? error.message : t('labels.updateFailed')
    } finally {
      labelDialogBusy.value = false
    }
  }

  return {
    cancelLabelRename,
    closeLabelDialog,
    deleteAccountLabel,
    isLabelApplied,
    labelDialogBusy,
    labelDialogError,
    labelDialogLabels,
    labelDialogMessageIds,
    labelDialogMessages,
    labelDialogOpen,
    labelDialogSummary,
    labelDraftName,
    labelRenameSource,
    openLabelDialog,
    openLabelDialogForMessages,
    startRenameLabel,
    submitLabelDraft,
    toggleMessageLabel,
  }
}
