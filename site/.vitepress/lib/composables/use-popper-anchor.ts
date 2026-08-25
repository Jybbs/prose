import { ref, shallowRef, type Ref, type ShallowRef } from 'vue'

// Tracks the element a delegated popper points at. `key` changes whenever the
// reference does, so keying the popper on it mounts a fresh instance.
export function usePopperAnchor(): {
  aim       : (element: HTMLElement) => void
  key       : Ref<number>
  reference : () => HTMLElement
  target    : ShallowRef<HTMLElement | null>
} {
  const key    = ref(0)
  const target = shallowRef<HTMLElement | null>(null)

  const aim = (element: HTMLElement): void => {
    if (element === target.value) return
    key.value   += 1
    target.value = element
  }

  return { aim, key, reference: () => target.value!, target }
}
