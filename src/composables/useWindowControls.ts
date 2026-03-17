import { computed, onMounted, ref } from 'vue'

const getWindowControls = () => window.windowControls

export const useWindowControls = () => {
  const isSupported = computed(() => Boolean(getWindowControls()))
  const isMaximized = ref(false)

  const refreshMaximizedState = async () => {
    const controls = getWindowControls()

    if (!controls) {
      isMaximized.value = false
      return false
    }

    const nextState = await controls.isMaximized()
    isMaximized.value = nextState
    return nextState
  }

  const minimize = async () => {
    await getWindowControls()?.minimize()
  }

  const toggleMaximize = async () => {
    const controls = getWindowControls()

    if (!controls) {
      return
    }

    isMaximized.value = await controls.toggleMaximize()
  }

  const close = async () => {
    await getWindowControls()?.close()
  }

  onMounted(() => {
    void refreshMaximizedState()
  })

  return {
    close,
    isMaximized,
    isSupported,
    minimize,
    toggleMaximize,
  }
}
