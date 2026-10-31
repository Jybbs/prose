<script setup lang="ts">
import RuleCard from '../rules/RuleCard.vue'

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
    class="sandbox-plate"
    :data-family="rule.family || null"
    :aria-label="`Settings for ${rule.slug}`"
  >
    <div class="relative">
      <RuleCard v-if="card" :rule="card" :clickable="false" />
      <div v-else class="px-16 py-12">
        <span class="font-mono text-kicker-sm tracking-kicker text-family-or-brand-1 uppercase">{{ rule.slug }}</span>
      </div>
      <button
        type="button"
        class="sandbox-plate-close grid size-26 place-items-center rounded-sm border-1 hover:border-family-or-brand-1-55-divider hover:text-family-or-brand-1 focus-visible:outline-offset-1"
        aria-label="Close"
        @click="$emit('close')"
      >
        <svg class="glyph" viewBox="0 0 24 24" fill="none" aria-hidden="true">
          <path d="M6 6l12 12" />
          <path d="M18 6L6 18" />
        </svg>
      </button>
    </div>
    <div class="sandbox-plate-divider after:absolute after:-top-1 after:left-0 after:h-2 after:w-30 after:bg-family-or-brand-1" aria-hidden="true" />
    <div class="sandbox-plate-rows">
      <div v-for="facet in facets" :key="facet.key" class="col-span-full grid min-h-2-1rem grid-cols-subgrid items-center not-first:border-t-1 not-first:border-t-divider medium:gap-y-0-1rem medium:py-0-4rem">
        <span class="font-mono text-xxs font-500 tracking-0-02em text-family-or-brand-1 uppercase medium:col-start-1 medium:row-start-1">{{ facet.label }}</span>
        <span class="sandbox-plate-hint py-0-25rem text-xs leading-1-4 text-text-3 italic medium:col-start-1 medium:row-start-2 medium:py-0" v-html="facet.hintHtml" />
        <button
          v-if="facet.kind === 'bool'"
          type="button"
          role="switch"
          class="sandbox-plate-control sandbox-plate-switch relative h-22 w-40 rounded-pill border-none bg-text-3/40 transition-background focus-visible:outline-offset-1 aria-checked:bg-family-or-brand-1"
          :aria-checked="boolValue(facet)"
          :aria-label="facet.label"
          @click="sandbox.setFacet(rule.slug, facet, !boolValue(facet))"
        >
          <span class="sandbox-plate-switch-knob" />
        </button>
        <input
          v-else-if="facet.kind === 'int'"
          type="number"
          class="sandbox-plate-control sandbox-plate-number panel-number h-22 rounded-sm border-1 border-family-or-brand-1-55-divider bg-bg font-mono text-xs text-text-1 focus-visible:outline-offset-1"
          :value="numberValue(facet)"
          :aria-label="facet.label"
          @input="sandbox.setFacet(rule.slug, facet, Number(($event.target as HTMLInputElement).value))"
        >
        <input
          v-else
          type="text"
          class="sandbox-plate-control h-22 w-9rem rounded-sm border-1 border-family-or-brand-1-55-divider bg-bg px-0-4rem font-mono text-xs text-text-1 focus-visible:outline-offset-1"
          :value="textValue(facet)"
          :aria-label="facet.label"
          @change="writeText(facet, ($event.target as HTMLInputElement).value)"
        >
      </div>
    </div>
  </section>
</template>
