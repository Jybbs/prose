import { computed, defineComponent } from 'vue'
import type { PropType }             from 'vue'

import type { Emblem } from '../../lib/shared/emblems'

import { nibId, nibSize } from './nib'

// Draws an emblem's nib from the sprite, labeled as an image where `label` is
// set and hidden from assistive technology where it is not.
export const NibBadge = defineComponent({
  name  : 'NibBadge',
  props : {
    emblem : { required: true, type: Object as PropType<Emblem> },
    label  : String,
    size   : { required: true, type: Number }
  },
  setup(props) {
    const href = computed(() => `#${nibId(props.emblem.motif, nibSize(props.size))}`)

    return () => (
      <svg
        class="nib-badge"
        width={props.size}
        height={props.size}
        viewBox="0 0 24 24"
        role={props.label ? 'img' : undefined}
        aria-label={props.label}
        aria-hidden={props.label ? undefined : 'true'}
      ><use href={href.value} /></svg>
    )
  }
})
