import { defineComponent }       from 'vue'
import type { PropType, VNode } from 'vue'

import type { RenderedLine } from '../../../lib/reference/run-summary'

// Draws one run's summary line in a terminal frame, holding the height of the
// `reserve` line where one is given so a longer line wraps without moving the page.
export const RunSummary = defineComponent({
  name  : 'RunSummary',
  props : {
    line    : { required: true, type: Object as PropType<RenderedLine> },
    reserve : { default: null, type: Object as PropType<RenderedLine | null> }
  },
  setup(props, { slots }) {
    function lineOf(line: RenderedLine): VNode {
      return (
        <div class="flex items-baseline gap-0-65em">
          {line.anchor ? (
            <span
              aria-hidden="true"
              class="run-summary-anchor shrink-0 basis-1-4em text-center data-ube:text-ube"
              data-ube={line.anchorUbe || undefined}
            >{line.anchor}</span>
          ) : null}
          <span
            class="run-summary-msg data-[tint=apricot]:text-apricot-code-45 data-[tint=celadon]:text-celadon-code-45"
            data-tint={line.countTint || undefined}
          >{line.text}</span>
        </div>
      )
    }

    return () => (
      <div class="overflow-hidden panel bg-code-block font-mono text-md">
        <div class="flex items-center gap-7 border-b-1 border-b-divider bg-bg-soft px-14 py-9">
          {Array.from({ length: 3 }, (_, index) => index + 1).map(n => <span aria-hidden="true" class="size-11 rounded-half bg-text-3 opacity-35" key={n} />)}
          <span aria-hidden="true" class="ms-6 text-xs tracking-0-05em text-text-3">prose</span>
          {slots.bar?.()}
        </div>
        <div class="grid min-h-1-5em px-22 py-18 leading-1-5">
          <div class="col-start-1 row-start-1">{lineOf(props.line)}</div>
          {props.reserve ? <div aria-hidden="true" class="invisible col-start-1 row-start-1">{lineOf(props.reserve)}</div> : null}
        </div>
      </div>
    )
  }
})
