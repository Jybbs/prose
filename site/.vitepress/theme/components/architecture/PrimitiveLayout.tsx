import { defineComponent } from 'vue'
import type { PropType }   from 'vue'

import { PrimitivesComposition } from './PrimitivesComposition'
import { DocHeading }            from '../../../kit'

import type { PrimitiveSlug } from '../../../lib/shared/registries'

export const PrimitiveLayout = defineComponent({
  name  : 'PrimitiveLayout',
  props : {
    primitive : { required: true, type: null as unknown as PropType<PrimitiveSlug> }
  },
  setup(props, { slots }) {
    return () => (
      <>
        <PrimitivesComposition initialFocus={props.primitive} />
        {slots.lead?.()}
        {slots.default?.()}
        <DocHeading id="related" title="Related" />
        {slots.related?.()}
      </>
    )
  }
})
