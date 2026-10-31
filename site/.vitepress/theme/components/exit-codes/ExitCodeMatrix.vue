<script setup lang="ts">
import { useTabSelect }  from '../../../lib/composables/use-tab-select'
import { data as codes } from '../../../lib/exit-codes/exit-codes.data'
import InlineProse       from '../base/InlineProse.vue'

const { active: selectedRow, selected } = useTabSelect(codes, c => c.code)

// Opens each outcome's band on the first code that reports it.
const heads = (at: number) => at === 0 || codes[at - 1]!.outcome !== codes[at]!.outcome
</script>

<template>
  <div class="exit-codes-spread">
    <nav class="exit-codes-index" aria-label="Exit codes">
      <template v-for="(row, at) in codes" :key="row.code">
        <header v-if="heads(at)" class="exit-codes-band-head band-head" :data-outcome="row.outcome">
          <span class="band-name">{{ row.band }}</span>
          <span class="band-rule" aria-hidden="true"></span>
        </header>
        <button
          :class="{ active: row.code === selected }"
          class="exit-code-index-row"
          :data-outcome="row.outcome"
          type="button"
          @focus="selected = row.code"
          @mouseenter="selected = row.code"
        >
          <span class="exit-code-index-num">{{ row.code }}</span>
          <span class="exit-code-index-label">{{ row.label }}</span>
          <span class="exit-code-index-leader" aria-hidden="true" />
        </button>
      </template>
    </nav>
    <article class="exit-code-entry" :data-outcome="selectedRow.outcome" aria-live="polite">
      <header class="exit-code-entry-head">
        <span class="exit-code-entry-numeral" aria-hidden="true">{{ selectedRow.code }}</span>
        <span class="exit-code-entry-label">{{ selectedRow.label }}</span>
      </header>
      <p class="exit-code-entry-summary">{{ selectedRow.summary }}</p>
      <dl class="exit-code-entry-facts">
        <template v-for="fact in selectedRow.facts" :key="fact.label">
          <dt>{{ fact.label }}</dt>
          <dd><InlineProse :nodes="fact.nodes" /></dd>
        </template>
      </dl>
    </article>
  </div>
</template>
