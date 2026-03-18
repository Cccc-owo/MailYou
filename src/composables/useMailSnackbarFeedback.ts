import { computed, ref } from 'vue'
import { useContextMenu } from '@/composables/useContextMenu'

interface SnackbarFeedbackOptions {
  t: (key: string) => string
  retrySend: () => Promise<void>
  retryLastAction: () => Promise<void>
  clearComposerFeedback: () => void
  clearMessagesError: () => void
  clearMailboxError: () => void
  getFallbackCopyText: () => string
}

export const useMailSnackbarFeedback = ({
  t,
  retrySend,
  retryLastAction,
  clearComposerFeedback,
  clearMessagesError,
  clearMailboxError,
  getFallbackCopyText,
}: SnackbarFeedbackOptions) => {
  const snackbarCtx = useContextMenu()
  const snackbarHasSelection = ref(false)

  const composerErrorActions = computed(() => [
    { key: 'retry', label: t('common.retry') },
    { key: 'close', icon: 'mdi-close', size: 'small' as const },
  ])

  const messagesErrorActions = computed(() => [
    { key: 'retry', label: t('common.retry') },
    { key: 'dismiss', label: t('common.dismiss') },
  ])

  const dismissOnlyActions = computed(() => [
    { key: 'dismiss', label: t('common.dismiss') },
  ])

  const openSnackbarMenu = (event: MouseEvent) => {
    snackbarHasSelection.value = Boolean(window.getSelection()?.toString())
    snackbarCtx.open(event)
  }

  const snackbarSelectAll = () => {
    const selection = window.getSelection()
    if (!selection) return

    const snackbar = document.querySelector('.mail-shell__snackbar--error .v-snackbar__wrapper')
    if (!snackbar) return

    const range = document.createRange()
    range.selectNodeContents(snackbar)
    selection.removeAllRanges()
    selection.addRange(range)
  }

  const snackbarCopy = () => {
    const selection = window.getSelection()
    if (selection && selection.toString()) {
      navigator.clipboard.writeText(selection.toString())
      return
    }

    navigator.clipboard.writeText(getFallbackCopyText())
  }

  const handleComposerErrorAction = (key: string) => {
    if (key === 'retry') {
      void retrySend()
      return
    }

    clearComposerFeedback()
  }

  const handleMessagesErrorAction = (key: string) => {
    if (key === 'retry') {
      void retryLastAction()
      return
    }

    clearMessagesError()
  }

  const handleMailboxErrorAction = () => {
    clearMailboxError()
  }

  return {
    composerErrorActions,
    dismissOnlyActions,
    handleComposerErrorAction,
    handleMailboxErrorAction,
    handleMessagesErrorAction,
    messagesErrorActions,
    openSnackbarMenu,
    snackbarCopy,
    snackbarCtx,
    snackbarHasSelection,
    snackbarSelectAll,
  }
}
