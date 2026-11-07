import { computed, defineComponent, ref } from 'vue'

import { RunSummary }       from './RunSummary'
import { RunSummarySelect } from './RunSummarySelect'

import * as runSummary from '../../../lib/reference/run-summary'

export const RunSummaryExplorer = defineComponent({
  name: 'RunSummaryExplorer',
  setup() {
    const outcomeId = ref('clean')
    const quietId   = ref('full')
    const streamId  = ref('tty')

    const line  = computed(() => runSummary.resolveSelection(outcomeId.value, quietId.value, streamId.value))
    const gloss = computed(() => runSummary.glossFor(outcomeId.value, quietId.value, streamId.value))

    const outcomeOpts = computed<runSummary.SelectOption[]>(() => runSummary.OUTCOMES.map(o => ({
      id      : o.key,
      mono    : o.args,
      preview : runSummary.resolveSelection(o.key, quietId.value, streamId.value)
    })))

    const quietOpts = computed<runSummary.SelectOption[]>(() => runSummary.QUIET_OPTIONS.map(q => ({
      ...q,
      preview: runSummary.resolveSelection(outcomeId.value, q.id, streamId.value)
    })))

    const streamOpts = computed<runSummary.SelectOption[]>(() => runSummary.STREAM_OPTIONS.map(s => ({
      ...s,
      preview: runSummary.resolveSelection(outcomeId.value, quietId.value, s.id)
    })))

    return () => (
      <div class="my-20 flex flex-col gap-12">
        <span class="kicker">Build A Run</span>
        <div class="flex flex-wrap items-baseline gap-0-5em panel bg-code-block px-17 py-13 font-mono text-md leading-1-6">
          <span aria-hidden="true" class="whitespace-nowrap text-text-2 select-none">$ prose</span>
          <RunSummarySelect
            label="Run command"
            modelValue={outcomeId.value}
            onUpdate:modelValue={next => { outcomeId.value = next }}
            options={outcomeOpts.value}
          />
          <span aria-hidden="true" class="text-text-3 select-none">.</span>
          <RunSummarySelect
            label="Verbosity"
            modelValue={quietId.value}
            onUpdate:modelValue={next => { quietId.value = next }}
            options={quietOpts.value}
          />
          <RunSummarySelect
            label="Output stream"
            modelValue={streamId.value}
            onUpdate:modelValue={next => { streamId.value = next }}
            options={streamOpts.value}
          />
        </div>
        <RunSummary line={line.value}>
          {{
            bar: () => <span class="run-summary-caption ms-10 min-w-0 overflow-hidden border-s-1 border-s-divider ps-10 font-base text-md leading-1-4 text-ellipsis whitespace-nowrap text-text-3 italic">{gloss.value}</span>
          }}
        </RunSummary>
      </div>
    )
  }
})
