import { defineComponent, h } from 'vue'
import type { PropType }      from 'vue'

import { SCROLL_STOP } from '../../lib/shared/constants'

// Renders an element that scrolls with a tab stop of its own, so a keyboard
// scrolls it whether or not its content overflows at the width the page loads at.
export const ScrollRegion = defineComponent({
  name  : 'ScrollRegion',
  props : {
    as   : { default: 'div', type: String as PropType<'aside' | 'div' | 'pre' | 'table'> },
    html : { default: undefined, type: String }
  },
  setup(props, { slots }) {
    const attrs = { ...SCROLL_STOP, class: 'focus-visible:focus-ring focus-visible:-outline-offset-1' }

    return () => props.html === undefined
      ? h(props.as, attrs, slots.default?.())
      : h(props.as, { ...attrs, innerHTML: props.html })
  }
})
