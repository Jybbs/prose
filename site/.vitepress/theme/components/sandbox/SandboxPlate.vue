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
    <div class="sandbox-plate-specimen">
      <RuleCard v-if="card" :rule="card" :clickable="false" />
      <div v-else class="sandbox-plate-stub">
        <span class="sandbox-plate-stub-slug">{{ rule.slug }}</span>
      </div>
      <button
        type="button"
        class="panel-seat panel-seat-sm sandbox-plate-close"
        aria-label="Close"
        @click="$emit('close')"
      >
        <svg class="glyph" viewBox="0 0 24 24" fill="none" aria-hidden="true">
          <path d="M6 6l12 12" />
          <path d="M18 6L6 18" />
        </svg>
      </button>
    </div>
    <div class="sandbox-plate-divider" aria-hidden="true" />
    <div class="sandbox-plate-rows">
      <div v-for="facet in facets" :key="facet.key" class="sandbox-plate-row">
        <span class="sandbox-plate-label">{{ facet.label }}</span>
        <span class="sandbox-plate-hint" v-html="facet.hintHtml" />
        <button
          v-if="facet.kind === 'bool'"
          type="button"
          role="switch"
          class="sandbox-plate-control sandbox-plate-switch"
          :aria-checked="boolValue(facet)"
          :aria-label="facet.label"
          @click="sandbox.setFacet(rule.slug, facet, !boolValue(facet))"
        >
          <span class="sandbox-plate-switch-knob" />
        </button>
        <input
          v-else-if="facet.kind === 'int'"
          type="number"
          class="panel-number sandbox-plate-control sandbox-plate-number"
          :value="numberValue(facet)"
          :aria-label="facet.label"
          @input="sandbox.setFacet(rule.slug, facet, Number(($event.target as HTMLInputElement).value))"
        >
        <input
          v-else
          type="text"
          class="sandbox-plate-control sandbox-plate-text"
          :value="textValue(facet)"
          :aria-label="facet.label"
          @change="writeText(facet, ($event.target as HTMLInputElement).value)"
        >
      </div>
    </div>
  </section>
</template>
