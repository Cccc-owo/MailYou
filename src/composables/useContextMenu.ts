import { ref, type Ref } from 'vue'

export function useContextMenu<T = undefined>() {
  const isOpen = ref(false)
  const x = ref(0)
  const y = ref(0)
  const target = ref<T | undefined>() as Ref<T | undefined>

  const open = (event: MouseEvent, data?: T) => {
    event.preventDefault()
    x.value = event.clientX
    y.value = event.clientY
    target.value = data
    isOpen.value = true
  }

  return { isOpen, x, y, target, open }
}
