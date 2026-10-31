<script setup lang="ts">
import { data as codes } from '../../../lib/exit-codes/exit-codes.data'
import { useTabSelect }  from '../../../lib/composables/use-tab-select'
import InlineProse       from '../base/InlineProse.vue'

const { active: selectedRow, selected } = useTabSelect(codes, c => c.code)
</script>

<template>
  <div class="my-1-5rem grid grid-cols-280-1fr items-grid-start gap-24 tablet:grid-cols-1fr">
    <nav class="flex flex-col gap-4 py-4 pr-18 pl-4 not-tablet:border-r-1 not-tablet:border-r-divider tablet:border-b-1 tablet:border-r-none tablet:border-b-divider tablet:pe-0 tablet:pb-16" aria-label="Exit codes">
      <button
        v-for="row in codes"
        :key="row.code"
        :data-exit-code="row.code"
        :class="row.code === selected ? 'bg-exit-code-accent/12 text-text-1' : 'text-text-2 hover:bg-exit-code-accent/6 hover:text-text-1'"
        class="exit-code-index-row grid grid-cols-24-auto-1fr-10 items-baseline gap-14 border-none p-12 text-left transition-background-and-color"
        type="button"
        @focus="selected = row.code"
        @mouseenter="selected = row.code"
      >
        <span class="text-right font-mono text-1rem text-exit-code-accent lining-nums tabular-nums">{{ row.code }}</span>
        <span class="font-display text-md italic">{{ row.label }}</span>
        <span class="mx-4 leader" aria-hidden="true" />
        <span class="size-8 self-center bg-exit-code-accent" aria-hidden="true" />
      </button>
    </nav>
    <article class="border-l-2 border-l-exit-code-accent ps-18 tablet:ps-16" :data-exit-code="selectedRow.code" aria-live="polite">
      <header class="mb-18 flex items-center gap-14">
        <span class="font-mono text-2-6rem leading-1 font-500 text-exit-code-accent lining-nums tabular-nums" aria-hidden="true">{{ selectedRow.code }}</span>
        <span class="exit-code-entry-label font-display text-1-5rem leading-1 font-500 text-text-1">{{ selectedRow.label }}</span>
      </header>
      <p class="exit-code-entry-summary max-w-60ch text-md text-text-2">{{ selectedRow.summary }}</p>
      <ul class="m-0 list-none p-0">
        <li v-for="(nodes, idx) in selectedRow.detailNodes" :key="idx" class="relative my-8 ps-18 text-0-84rem leading-1-55 text-text-2 before:absolute before:top-0-45em before:left-0 before:text-0-6em before:text-exit-code-accent before:content-diamond"><InlineProse :nodes="nodes" /></li>
      </ul>
    </article>
  </div>
</template>
