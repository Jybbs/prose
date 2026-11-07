<script setup lang="ts">
import { onKeyStroke }   from '@vueuse/core'
import { computed, ref } from 'vue'

import { NibBadge } from '../../../kit'
import { RuleCard } from '../rules/RuleCard'
import SandboxPlate from './SandboxPlate.vue'
import SandboxRuler from './SandboxRuler.vue'

import * as chipPanel        from '../../../lib/composables/use-chip-panel'
import type { ProseSandbox } from '../../../lib/composables/use-prose-sandbox'
import { data as rules }     from '../../../lib/rules/rules.data'
import { familyEmblem }      from '../../../lib/shared/emblems'

const props = defineProps<{
  sandbox : ProseSandbox
}>()

defineEmits<{ dragging: [key: string, hue: string], preview: [key: string, value: number] }>()

const {
  enabledFacet, isOn, lengthValue, openFacets, openSlug, ruleData,
  setLength, setPanel, subFacets, toggle, visible, visibleLengths
} = chipPanel.useChipPanel(props.sandbox, rules.bySlug)

const hoveredSlug = ref('')

const hoverCard = computed(() => ruleData(hoveredSlug.value))

// The idle banner ghost-renders a card so the hint state holds the exact
// height a hovered card will take, whatever the viewport wraps it to.
const idleCard = computed(() => ruleData(visible.value[0]?.slug ?? ''))

const allOff = computed(() =>
  visible.value.length > 0 && visible.value.every(rule => !isOn(rule)))

function toggleAll(): void {
  const next = allOff.value
  for (const rule of visible.value) props.sandbox.setFacet(rule.slug, enabledFacet(rule), next)
}
const openRule   = computed(() => visible.value.find(rule => rule.slug === openSlug.value) ?? null)
const pinnedCard = computed(() => openRule.value ? ruleData(openRule.value.slug) : null)

// Escape closes the plate from anywhere on the page.
onKeyStroke('Escape', () => { openSlug.value = '' })

function hover(rule: chipPanel.RuleControl): void {
  hoveredSlug.value = rule.slug
}
</script>

<template>
  <div class="flex flex-col gap-0-85rem">
    <SandboxRuler
      :lengths="visibleLengths"
      :value-of="lengthValue"
      @set-length="setLength"
      @dragging="(key, hue) => $emit('dragging', key, hue)"
      @preview="(key, value) => $emit('preview', key, value)"
    />

    <div class="flex flex-wrap items-center gap-0-3rem">
      <button
        v-if="visible.length > 0"
        type="button"
        class="panel-seat size-26 transition-colors aria-pressed:text-text-3"
        :aria-pressed="allOff"
        :title="allOff ? 'Turn every rule on' : 'Turn every rule off'"
        :aria-label="allOff ? 'Turn every rule on' : 'Turn every rule off'"
        @click="toggleAll"
      >
        <svg class="size-13 glyph-stroke" viewBox="0 0 24 24" fill="none" aria-hidden="true">
          <path d="M12 3v8" />
          <path d="M6.3 6.6a8 8 0 1 0 11.4 0" />
        </svg>
      </button>
      <div
        v-for="rule in visible"
        :key="rule.slug"
        class="group relative inline-flex items-center rounded-sm data-active:z-5 sandbox-tags-tag bg-bg"
        :data-family="rule.family || null"
        :data-off="!isOn(rule) || undefined"
        :data-active="rule.slug === (openSlug || hoveredSlug) || undefined"
        @mouseover="hover(rule)"
        @focusin="hover(rule)"
      >
        <button type="button" class="inline-flex items-center gap-5 border-none px-7 py-2" @click="toggle(rule)">
          <NibBadge v-if="rule.family" :emblem="familyEmblem(rule.family)" :size="12" />
          <span class="font-mono text-0-68rem tracking-0-05em text-text-1 uppercase group-data-off:text-text-2 group-data-off:line-through group-data-off:decoration-text-2">{{ rule.slug }}</span>
        </button>
        <button
          v-if="subFacets(rule).length > 0"
          type="button"
          class="inline-flex items-center border-none py-2 pr-6 pl-3 text-text-3 transition-color-and-opacity disabled:cursor-default disabled:opacity-40 not-disabled:hover:text-family-nib aria-expanded:text-family-nib"
          data-gear
          :disabled="!isOn(rule)"
          :aria-expanded="openSlug === rule.slug"
          :aria-label="`Settings for ${rule.slug}`"
          @click="openFacets(rule)"
        >
          <svg class="size-15 glyph-stroke" viewBox="0 0 24 24" fill="none" aria-hidden="true">
            <path
              d="M10.3 4.3c.4-1.8 2.9-1.8 3.4 0a1.7 1.7 0 0 0 2.5 1.1c1.6-.9 3.3.8 2.4 2.4a1.7 1.7 0 0 0 1 2.5c1.8.4 1.8 2.9 0 3.4a1.7 1.7 0 0 0-1 2.5c.9 1.6-.8 3.3-2.4 2.4a1.7 1.7 0 0 0-2.5 1c-.5 1.8-3 1.8-3.4 0a1.7 1.7 0 0 0-2.5-1c-1.6.9-3.3-.8-2.4-2.4a1.7 1.7 0 0 0-1-2.5c-1.8-.5-1.8-3 0-3.4a1.7 1.7 0 0 0 1-2.5c-.9-1.6.8-3.3 2.4-2.4a1.7 1.7 0 0 0 2.5-1z"
            />
            <path d="M9 12a3 3 0 1 0 6 0a3 3 0 0 0-6 0" />
          </svg>
        </button>
      </div>
    </div>

    <aside class="sandbox-tags-banner flex max-h-tags-banner min-h-5-5rem overflow-y-auto panel bg-bg-soft px-1-25rem py-0-9rem">
      <SandboxPlate
        v-if="openRule"
        :ref="setPanel"
        :card="pinnedCard"
        :facets="subFacets(openRule)"
        :rule="openRule"
        :sandbox="sandbox"
        @close="openSlug = ''"
      />
      <RuleCard v-else-if="hoverCard" :key="hoverCard.slug" :rule="hoverCard" />
      <div v-else-if="idleCard" class="relative flex w-full">
        <RuleCard class="invisible" :rule="idleCard" :clickable="false" aria-hidden="true" />
        <p class="absolute inset-0 m-0 grid place-items-center text-center text-sm text-text-3">Hover a rule to read what it does.</p>
      </div>
      <p v-else class="text-center text-sm text-text-3">Hover a rule to read what it does.</p>
    </aside>
  </div>
</template>
