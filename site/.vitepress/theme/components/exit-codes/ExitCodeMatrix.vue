<script setup lang="ts">
import { data as codes } from '../../../lib/exit-codes/exit-codes.data'
import { useTabSelect }  from '../../../lib/composables/use-tab-select'
import InlineProse       from '../base/InlineProse.vue'

const { active: selectedRow, selected } = useTabSelect(codes, c => c.code)
</script>

<template>
  <div class="exit-codes-spread">
    <nav class="exit-codes-index" aria-label="Exit codes">
      <button
        v-for="row in codes"
        :key="row.code"
        :data-exit-code="row.code"
        :class="{ active: row.code === selected }"
        class="exit-code-index-row"
        type="button"
        @focus="selected = row.code"
        @mouseenter="selected = row.code"
      >
        <span class="exit-code-index-num">{{ row.code }}</span>
        <span class="exit-code-index-label">{{ row.label }}</span>
        <span class="exit-code-index-leader" aria-hidden="true" />
        <span class="exit-code-index-mark" aria-hidden="true" />
      </button>
    </nav>
    <article class="exit-code-entry" :data-exit-code="selectedRow.code" aria-live="polite">
      <header class="exit-code-entry-head">
        <span class="exit-code-entry-numeral" aria-hidden="true">{{ selectedRow.code }}</span>
        <span class="exit-code-entry-label">{{ selectedRow.label }}</span>
      </header>
      <p class="exit-code-entry-summary">{{ selectedRow.summary }}</p>
      <ul class="exit-code-entry-details">
        <li v-for="(nodes, idx) in selectedRow.detailNodes" :key="idx"><InlineProse :nodes="nodes" /></li>
      </ul>
    </article>
  </div>
</template>
