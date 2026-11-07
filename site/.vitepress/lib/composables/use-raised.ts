import { onKeyStroke }                                      from '@vueuse/core'
import { computed, shallowRef, type ComputedRef, type Ref } from 'vue'

interface Listeners {
  onClick      : () => void
  onFocusin    : (event: FocusEvent) => void
  onFocusout   : () => void
  onMouseenter : () => void
  onMouseleave : () => void
}

interface Raised<T> {
  on      : (item: T) => Listeners
  pinned  : Readonly<Ref<T | null>>
  release : () => void
  shown   : ComputedRef<T | null>
}

// Holds the item a reader points at, focuses from the keyboard, or pins with a
// click, the pointer outranking the focus and the focus the pin. A second click
// on the pinned item, the Escape key, or `release` releases the pin, and `on`
// returns the listeners an element naming an item binds.
export function useRaised<T>(): Raised<T> {
  const focused = shallowRef<T | null>(null)
  const pinned  = shallowRef<T | null>(null)
  const pointed = shallowRef<T | null>(null)

  // Counts only a focus the browser draws a ring for, since a click in most
  // browsers also focuses the element it unpins, which would keep that item
  // raised.
  const focus = (event: FocusEvent, item: T): void => {
    if ((event.target as Element).matches(':focus-visible')) focused.value = item
  }

  onKeyStroke('Escape', () => {
    focused.value = null
    pinned.value  = null
  })

  return {
    pinned  : pinned,
    release : () => { pinned.value = null },
    shown   : computed(() => pointed.value ?? focused.value ?? pinned.value),
    on      : item => ({
      onClick      : () => { pinned.value = pinned.value === item ? null : item },
      onFocusin    : event => focus(event, item),
      onFocusout   : () => { focused.value = null },
      onMouseenter : () => { pointed.value = item },
      onMouseleave : () => { pointed.value = null }
    })
  }
}
