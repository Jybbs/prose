import { defineComponent } from 'vue'
import type { PropType }   from 'vue'

// Renders the small uppercase line that opens a section.
export const Kicker = defineComponent({
  name  : 'Kicker',
  props : { variant: { default: 'plain', type: String as PropType<'plain' | 'rule' | 'wide'> } },
  setup(props, { slots }) {
    return () => <p class="kicker" data-variant={props.variant}>{slots.default?.()}</p>
  }
})
