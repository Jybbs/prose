<script setup lang="ts">
import { computed, ref } from 'vue'

import RunSummary       from './RunSummary.vue'
import RunSummarySelect from './RunSummarySelect.vue'

import {
  glossFor,
  OUTCOMES,
  QUIET_OPTIONS,
  resolveSelection,
  type SelectOption,
  STREAM_OPTIONS
} from './run-summary'

const outcomeId = ref('clean')
const quietId   = ref('full')
const streamId  = ref('tty')

const line  = computed(() => resolveSelection(outcomeId.value, quietId.value, streamId.value))
const gloss = computed(() => glossFor(outcomeId.value, quietId.value, streamId.value))

const outcomeOpts = computed<SelectOption[]>(() => OUTCOMES.map(o => ({
  id      : o.key,
  mono    : o.args,
  preview : resolveSelection(o.key, quietId.value, streamId.value)
})))

const quietOpts = computed<SelectOption[]>(() => QUIET_OPTIONS.map(q => ({
  id      : q.id,
  mono    : q.mono,
  preview : resolveSelection(outcomeId.value, q.id, streamId.value)
})))

const streamOpts = computed<SelectOption[]>(() => STREAM_OPTIONS.map(s => ({
  id      : s.id,
  mono    : s.mono,
  preview : resolveSelection(outcomeId.value, quietId.value, s.id)
})))
</script>

<template>
  <div class="run-summary-explorer">
    <span class="kicker run-summary-explorer-kicker">Build A Run</span>
    <div class="run-summary-cmd">
      <span class="run-summary-cmd-prompt" aria-hidden="true">$ prose</span>
      <RunSummarySelect v-model="outcomeId" :options="outcomeOpts" aria-label="Run command" />
      <span class="run-summary-cmd-path" aria-hidden="true">.</span>
      <RunSummarySelect v-model="quietId" :options="quietOpts" aria-label="Verbosity" />
      <RunSummarySelect v-model="streamId" :options="streamOpts" aria-label="Output stream" />
    </div>
    <RunSummary :line="line">
      <template #bar>
        <span class="run-summary-caption">{{ gloss }}</span>
      </template>
    </RunSummary>
  </div>
</template>
