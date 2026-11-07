<script setup lang="ts">
import { RuleCard } from '../rules/RuleCard'

import type { Facet, RuleControl } from '../../../lib/composables/use-chip-panel'
import type { ProseSandbox }       from '../../../lib/composables/use-prose-sandbox'
import type { RenderedRule }       from '../../../lib/rules/rules.data'

const props = defineProps<{
  card    : RenderedRule | null
  facets  : readonly Facet[]
  rule    : RuleControl
  sandbox : ProseSandbox
}>()

defineEmits<{ close: [] }>()

function boolValue(facet: Facet): boolean {
  return props.sandbox.facetValue(props.rule.slug, facet) === true
}

function numberValue(facet: Facet): number {
  return props.sandbox.facetValue(props.rule.slug, facet) as number
}

function textValue(facet: Facet): string {
  const value = props.sandbox.facetValue(props.rule.slug, facet)
  return Array.isArray(value) ? value.join(', ') : String(value)
}

function writeText(facet: Facet, raw: string): void {
  const value = facet.kind === 'stringList'
    ? raw.split(',').map(part => part.trim()).filter(Boolean)
    : raw
  props.sandbox.setFacet(props.rule.slug, facet, value)
}
</script>

<template>
  <section
    class="sandbox-plate w-full min-w-0 rounded-sm border-1 border-text-1-40-divider bg-text-1-4-panel-wash-card"
    :data-family="rule.family || null"
    :aria-label="`Settings for ${rule.slug}`"
  >
    <div class="relative">
      <RuleCard v-if="card" :rule="card" :clickable="false" />
      <div v-else class="px-16 py-12">
        <span class="font-mono text-kicker-sm tracking-kicker text-text-1 uppercase">{{ rule.slug }}</span>
      </div>
      <button
        type="button"
        class="sandbox-plate-close absolute top-1/2 right-16 grid size-26 animate-plate-settle place-items-center rounded-sm border-1 border-divider bg-bg text-text-3 transition-colors hover:border-text-1-55-divider hover:text-text-1 focus-visible:outline-offset-1"
        aria-label="Close"
        @click="$emit('close')"
      >
        <svg class="glyph" viewBox="0 0 24 24" fill="none" aria-hidden="true">
          <path d="M6 6l12 12" />
          <path d="M18 6L6 18" />
        </svg>
      </button>
    </div>
    <div class="relative mx-16 h-1 origin-left scale-x-0 animate-plate-draw bg-divider after:absolute after:-top-1 after:left-0 after:h-2 after:w-30 after:bg-text-1 motion-reduce:scale-x-100" aria-hidden="true" />
    <div class="grid animate-plate-settle grid-cols-max-1fr-max gap-x-1-1rem px-16 pb-8 max-medium:grid-cols-1fr-max">
      <div v-for="facet in facets" :key="facet.key" class="col-span-full grid min-h-2-1rem grid-cols-subgrid items-center not-first:border-t-1 not-first:border-t-divider max-medium:gap-y-0-1rem max-medium:py-0-4rem">
        <span class="font-mono text-xxs font-500 tracking-0-02em text-text-1 uppercase max-medium:col-start-1 max-medium:row-start-1">{{ facet.label }}</span>
        <span class="sandbox-plate-hint py-0-25rem text-xs leading-1-4 text-text-3 italic max-medium:col-start-1 max-medium:row-start-2 max-medium:py-0" v-html="facet.hintHtml" />
        <button
          v-if="facet.kind === 'bool'"
          type="button"
          role="switch"
          class="group/switch relative h-22 w-40 justify-self-end rounded-pill border-none bg-text-3/40 transition-colors focus-visible:outline-offset-1 aria-checked:bg-text-1 max-medium:col-2 max-medium:row-span-2 max-medium:row-start-1"
          :aria-checked="boolValue(facet)"
          :aria-label="facet.label"
          @click="sandbox.setFacet(rule.slug, facet, !boolValue(facet))"
        >
          <span class="absolute top-3 left-3 size-16 rounded-half bg-bg transition-transform group-aria-checked/switch:translate-x-18" />
        </button>
        <input
          v-else-if="facet.kind === 'int'"
          type="number"
          class="panel-number h-22 w-40 justify-self-end rounded-sm border-1 border-text-1-55-divider bg-bg font-mono text-xs leading-20 text-text-1 tabular-nums focus-visible:outline-offset-1 max-medium:col-2 max-medium:row-span-2 max-medium:row-start-1"
          :value="numberValue(facet)"
          :aria-label="facet.label"
          @input="sandbox.setFacet(rule.slug, facet, Number(($event.target as HTMLInputElement).value))"
        >
        <input
          v-else
          type="text"
          class="h-22 w-9rem justify-self-end rounded-sm border-1 border-text-1-55-divider bg-bg px-0-4rem font-mono text-xs text-text-1 focus-visible:outline-offset-1 max-medium:col-2 max-medium:row-span-2 max-medium:row-start-1"
          :value="textValue(facet)"
          :aria-label="facet.label"
          @change="writeText(facet, ($event.target as HTMLInputElement).value)"
        >
      </div>
    </div>
  </section>
</template>
