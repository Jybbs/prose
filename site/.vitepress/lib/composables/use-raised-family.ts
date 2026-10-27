import { onKeyStroke }                                    from '@vueuse/core'
import { computed, shallowRef, type ComputedRef, type Ref } from 'vue'

import type { GlossaryFamily } from '../shared/registries'

type Held = GlossaryFamily | null

interface Listeners {
  onClick      : () => void
  onFocusin    : (event: FocusEvent) => void
  onFocusout   : () => void
  onMouseenter : () => void
  onMouseleave : () => void
}

interface RaisedFamily {
  on     : (family: GlossaryFamily) => Listeners
  pinned : Readonly<Ref<Held>>
  shown  : ComputedRef<Held>
}

// Holds the family a reader points at, focuses from the keyboard, or pins with
// a click, the pointer outranking the focus and the focus the pin. A second
// click on the pinned family or the Escape key releases the pin, and `on`
// returns the listeners an element naming a family binds.
export function useRaisedFamily(): RaisedFamily {
  const focused = shallowRef<Held>(null)
  const pinned  = shallowRef<Held>(null)
  const pointed = shallowRef<Held>(null)

  // Counts only a focus the browser draws a ring for, since a click also
  // focuses the key chip it unpins and would otherwise keep that family raised.
  const focus = (event: FocusEvent, family: GlossaryFamily): void => {
    if ((event.target as Element).matches(':focus-visible')) focused.value = family
  }

  onKeyStroke('Escape', () => {
    focused.value = null
    pinned.value  = null
  })

  return {
    pinned : pinned,
    shown  : computed(() => pointed.value ?? focused.value ?? pinned.value),
    on     : family => ({
      onClick      : () => { pinned.value = pinned.value === family ? null : family },
      onFocusin    : event => focus(event, family),
      onFocusout   : () => { focused.value = null },
      onMouseenter : () => { pointed.value = family },
      onMouseleave : () => { pointed.value = null }
    })
  }
}
