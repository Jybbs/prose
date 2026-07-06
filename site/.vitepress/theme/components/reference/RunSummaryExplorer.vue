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
  <div class="run-summary-explorer">
    <span class="kicker run-summary-explorer-kicker">Build A Run</span>
    <div class="run-summary-cmd panel">
      <span class="run-summary-cmd-prompt" aria-hidden="true">$ prose</span>
      <RunSummarySelect v-model="outcomeId" :options="outcomeOpts" label="Run command" />
      <span class="run-summary-cmd-path" aria-hidden="true">.</span>
      <RunSummarySelect v-model="quietId" :options="quietOpts" label="Verbosity" />
      <RunSummarySelect v-model="streamId" :options="streamOpts" label="Output stream" />
    </div>
    <RunSummary :line="line">
      <template #bar>
        <span class="run-summary-caption">{{ gloss }}</span>
      </template>
    </RunSummary>
  </div>
</template>
