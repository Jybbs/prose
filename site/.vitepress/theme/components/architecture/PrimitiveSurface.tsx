import { computed, defineComponent } from 'vue'
import type { PropType }             from 'vue'

import { ScrollRegion } from '../../../kit'

import { data as primitives } from '../../../lib/architecture/primitives-composition.data'

import type { PrimitiveStability } from '../../../lib/shared/registries'
import { InlineProse }             from '../base/InlineProse'

export const PrimitiveSurface = defineComponent({
  name  : 'PrimitiveSurface',
  props : {
    stability : { required: true, type: null as unknown as PropType<PrimitiveStability> }
  },
  setup(props) {
    const listed = computed(() =>
      primitives.entries.filter(entry => entry.stability === props.stability)
                        .toSorted((a, b) => a.slug.localeCompare(b.slug))
    )

    return () => (
      <ScrollRegion as="table">
        <thead>
          <tr>
            <th>Primitive</th>
            <th>Role</th>
          </tr>
        </thead>
        <tbody>
          {listed.value.map(row => (
            <tr key={row.slug}>
              <td>
                <InlineProse nodes={row.linkNodes} />
              </td>
              <td>
                <InlineProse nodes={row.summaryNodes} />
              </td>
            </tr>
          ))}
        </tbody>
      </ScrollRegion>
    )
  }
})
