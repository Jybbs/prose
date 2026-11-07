import { defineComponent } from 'vue'
import type { PropType }   from 'vue'

import type { InlineNode } from '../../../lib/markdown/inline-nodes'
import { InlineProse }     from '../base/InlineProse'

// Draws a fixture case's description above its panels, and nothing where the
// case carries none.
export const FixtureLead = defineComponent({
  name  : 'FixtureLead',
  props : {
    nodes : { default: undefined, type: Array as PropType<readonly InlineNode[]> }
  },
  setup(props) {
    return () => props.nodes ? (
      <div class="fixture-lead mb-1rem">
        <InlineProse nodes={props.nodes} />
      </div>
    ) : null
  }
})
