import { computed, ref } from 'vue'

interface UndoableAction {
  label: string
  undo: () => Promise<void>
  timer: ReturnType<typeof setTimeout>
}

export const useUndoableAction = (t: (key: string) => string) => {
  const undoableAction = ref<UndoableAction | null>(null)

  const undoActions = computed(() => [
    { key: 'undo', label: t('common.undo') },
    { key: 'dismiss', label: t('common.dismiss') },
  ])

  const performUndoable = (label: string, undoFn: () => Promise<void>) => {
    if (undoableAction.value) {
      clearTimeout(undoableAction.value.timer)
    }

    const timer = setTimeout(() => {
      undoableAction.value = null
    }, 5000)

    undoableAction.value = { label, undo: undoFn, timer }
  }

  const handleUndo = async () => {
    if (!undoableAction.value) return

    clearTimeout(undoableAction.value.timer)
    const action = undoableAction.value
    undoableAction.value = null
    await action.undo()
  }

  const dismissUndo = () => {
    if (!undoableAction.value) return

    clearTimeout(undoableAction.value.timer)
    undoableAction.value = null
  }

  const handleUndoAction = (key: string) => {
    if (key === 'undo') {
      void handleUndo()
      return
    }

    dismissUndo()
  }

  return {
    dismissUndo,
    handleUndo,
    handleUndoAction,
    performUndoable,
    undoActions,
    undoableAction,
  }
}
