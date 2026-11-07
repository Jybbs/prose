import { computed, defineComponent } from 'vue'

import { NibBadge } from '../../../kit'

import { BADGE, orderLine } from '../../../lib/rules/order-line'
import type { OrderMark }   from '../../../lib/rules/order-line'
import { data as pipeline } from '../../../lib/rules/pipeline.data'
import { familyEmblem }     from '../../../lib/shared/emblems'

const WIDTH = 176

// Draws the current rule's place in the pipeline as one line, the rules it runs
// after as badges above it and the rules running after it as badges below.
export const RuleOrderLine = defineComponent({
  name  : 'RuleOrderLine',
  props : {
    slug : { required: true, type: String }
  },
  setup(props) {
    const line  = computed(() => orderLine(pipeline.rules, props.slug, WIDTH))
    const label = computed(() => [
      line.value.earlier.length > 0 ? `runs after ${line.value.earlier.map(mark => mark.slug).join(', ')}` : '',
      line.value.later.length > 0 ? `${line.value.later.map(mark => mark.slug).join(', ')} run after it` : ''
    ].filter(Boolean).join(', and ') || 'no declared order')

    const badge = (mark: OrderMark) => (
      <g data-family={mark.family ?? undefined} key={mark.slug} transform={`translate(${mark.x - BADGE / 2} ${mark.y})`}>
        <title>{mark.slug}</title>
        {mark.family ? <NibBadge emblem={familyEmblem(mark.family)} size={BADGE} /> : <circle cx={BADGE / 2} cy={BADGE / 2} r={2} />}
      </g>
    )

    return () => (
      <svg
        aria-label={`Pipeline order: ${label.value}`}
        class="shrink-0 text-text-3"
        height={line.value.height}
        role="img"
        viewBox={`0 0 ${WIDTH} ${line.value.height}`}
        width={WIDTH}
      >
        <line stroke="currentColor" x1={0} x2={WIDTH} y1={line.value.lineY} y2={line.value.lineY} />
        <line
          class="text-brand-1"
          stroke="currentColor"
          stroke-width={2}
          x1={line.value.here}
          x2={line.value.here}
          y1={line.value.lineY - BADGE / 2}
          y2={line.value.lineY + BADGE / 2}
        />
        {line.value.earlier.map(badge)}
        {line.value.later.map(badge)}
      </svg>
    )
  }
})
