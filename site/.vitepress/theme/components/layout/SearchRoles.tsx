import { defaultDocument, useMutationObserver, whenever } from '@vueuse/core'
import { defineComponent, shallowRef }                    from 'vue'

const CARRIED = ['aria-selected', 'id']

// Rewrites the roles VitePress gives its local search box, which nest controls
// inside a `button` and a link inside each `option`. The box becomes a `dialog`,
// each result's link becomes the `option` itself, named by its own text, and each
// shortcut key takes the `img` role its label requires outside a `button`.
export const SearchRoles = defineComponent({
  name: 'SearchRoles',
  setup() {
    const box = shallowRef<HTMLElement | null>(null)

    // Vue writes the selection and the id back to the row, so each write moves on
    // to the row's link.
    function carry(): void {
      for (const link of box.value?.querySelectorAll('.result') ?? []) {
        const row = link.parentElement!
        row.setAttribute('role', 'none')
        link.setAttribute('role', 'option')
        link.removeAttribute('aria-label')
        for (const name of CARRIED) {
          const held = row.getAttributeNode(name)
          if (held) link.setAttributeNode(row.removeAttributeNode(held))
        }
      }
    }

    // The box teleports to the body as it opens.
    useMutationObserver(
      defaultDocument?.body,
      () => { box.value = document.querySelector<HTMLElement>('body > .VPLocalSearchBox') },
      { childList: true }
    )
    useMutationObserver(box, carry, { attributeFilter: CARRIED, childList: true, subtree: true })

    // Runs once the observer above attaches, covering every row rendered before it.
    whenever(box, opened => {
      opened.setAttribute('role', 'dialog')
      opened.removeAttribute('aria-expanded')
      opened.removeAttribute('aria-haspopup')
      for (const key of opened.querySelectorAll('kbd[aria-label]')) key.setAttribute('role', 'img')
      carry()
    }, { flush: 'post' })

    return () => null
  }
})
