<script setup lang="ts">
import { computed, ref } from 'vue'

import RunSummary       from './RunSummary.vue'
import RunSummarySelect from './RunSummarySelect.vue'

import * as runSummary from '../../../lib/reference/run-summary'

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
</script>

<template>
  <div class="my-20 flex flex-col gap-12">
    <span class="kicker">Build A Run</span>
    <div class="flex flex-wrap items-baseline gap-0-5em panel bg-code-block px-17 py-13 font-mono text-md leading-1-6">
      <span class="whitespace-nowrap text-text-2 select-none" aria-hidden="true">$ prose</span>
      <RunSummarySelect v-model="outcomeId" :options="outcomeOpts" label="Run command" />
      <span class="text-text-3 select-none" aria-hidden="true">.</span>
      <RunSummarySelect v-model="quietId" :options="quietOpts" label="Verbosity" />
      <RunSummarySelect v-model="streamId" :options="streamOpts" label="Output stream" />
    </div>
    <RunSummary :line="line">
      <template #bar>
        <span class="run-summary-caption ms-10 min-w-0 overflow-hidden border-s-1 border-s-divider ps-10 font-base text-md leading-1-4 text-ellipsis whitespace-nowrap text-text-3 italic">{{ gloss }}</span>
      </template>
    </RunSummary>
  </div>
</template>
