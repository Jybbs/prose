import { defineComponent, h } from 'vue'
import type { PropType }      from 'vue'

import { externalAttrs } from '../../lib/shared/links'

type ChipVariant = 'family-chip' | 'rule-chip'

// Renders a chip as a link where `href` is set and as a span where it is not.
export const Chip = defineComponent({
  name  : 'Chip',
  props : {
    family  : String,
    href    : String,
    title   : String,
    variant : { required: true, type: String as PropType<ChipVariant> }
  },
  setup(props, { slots }) {
    return () => h(
      props.href ? 'a' : 'span',
      {
        href          : props.href,
        title         : props.title,
        'data-family' : props.family,
        class         : ['chip', props.variant],
        ...externalAttrs(props.href)
      },
      slots.default?.()
    )
  }
})
