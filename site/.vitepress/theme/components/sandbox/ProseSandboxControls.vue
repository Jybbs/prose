<script setup lang="ts">
import { onKeyStroke }   from '@vueuse/core'
import { computed, ref } from 'vue'

import RuleCard     from '../rules/RuleCard.vue'
import SandboxPlate from './SandboxPlate.vue'
import SandboxRuler from './SandboxRuler.vue'

import * as chipPanel        from '../../../lib/composables/use-chip-panel'
import type { ProseSandbox } from '../../../lib/composables/use-prose-sandbox'
import { data as rules }     from '../../../lib/rules/rules.data'

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
  <div class="tags">
    <SandboxRuler
      :lengths="visibleLengths"
      :value-of="lengthValue"
      @set-length="setLength"
      @dragging="(key, hue) => $emit('dragging', key, hue)"
      @preview="(key, value) => $emit('preview', key, value)"
    />

    <div class="tags-cloud">
      <button
        v-if="visible.length > 0"
        type="button"
        class="tags-power"
        :aria-pressed="allOff"
        :title="allOff ? 'Turn every rule on' : 'Turn every rule off'"
        :aria-label="allOff ? 'Turn every rule on' : 'Turn every rule off'"
        @click="toggleAll"
      >
        <svg class="tags-power-icon glyph" viewBox="0 0 24 24" fill="none" aria-hidden="true">
          <path d="M12 3v8" />
          <path d="M6.3 6.6a8 8 0 1 0 11.4 0" />
        </svg>
      </button>
      <div
        v-for="rule in visible"
        :key="rule.slug"
        class="tags-tag"
        :data-family="rule.family || null"
        :data-on="isOn(rule)"
        :data-active="rule.slug === (openSlug || hoveredSlug)"
        @mouseover="hover(rule)"
        @focusin="hover(rule)"
      >
        <button type="button" class="tags-toggle" @click="toggle(rule)">
          <span class="tags-slug">{{ rule.slug }}</span>
        </button>
        <button
          v-if="subFacets(rule).length > 0"
          type="button"
          class="tags-gear"
          data-gear
          :disabled="!isOn(rule)"
          :aria-expanded="openSlug === rule.slug"
          :aria-label="`Settings for ${rule.slug}`"
          @click="openFacets(rule)"
        >
          <svg class="tags-gear-icon glyph" viewBox="0 0 24 24" fill="none" aria-hidden="true">
            <path
              d="M10.3 4.3c.4-1.8 2.9-1.8 3.4 0a1.7 1.7 0 0 0 2.5 1.1c1.6-.9 3.3.8 2.4 2.4a1.7 1.7 0 0 0 1 2.5c1.8.4 1.8 2.9 0 3.4a1.7 1.7 0 0 0-1 2.5c.9 1.6-.8 3.3-2.4 2.4a1.7 1.7 0 0 0-2.5 1c-.5 1.8-3 1.8-3.4 0a1.7 1.7 0 0 0-2.5-1c-1.6.9-3.3-.8-2.4-2.4a1.7 1.7 0 0 0-1-2.5c-1.8-.5-1.8-3 0-3.4a1.7 1.7 0 0 0 1-2.5c-.9-1.6.8-3.3 2.4-2.4a1.7 1.7 0 0 0 2.5-1z"
            />
            <path d="M9 12a3 3 0 1 0 6 0a3 3 0 0 0-6 0" />
          </svg>
        </button>
      </div>
    </div>

    <aside class="tags-banner">
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
      <div v-else-if="idleCard" class="tags-idle">
        <RuleCard class="tags-idle-ghost" :rule="idleCard" :clickable="false" aria-hidden="true" />
        <p class="tags-hint">Hover a rule to read what it does.</p>
      </div>
      <p v-else class="tags-hint">Hover a rule to read what it does.</p>
    </aside>
  </div>
</template>

<style scoped>
.tags {
  display        : flex;
  flex-direction : column;
  gap            : 0.85rem;
}

.tags-cloud {
  display     : flex;
  flex-wrap   : wrap;
  gap         : 0.3rem;
  align-items : center;
}

.tags-tag {
  position      : relative;
  display       : inline-flex;
  align-items   : center;
  border-radius : var(--prose-radius-sm);
  background    : color-mix(in srgb, var(--family-color, var(--vp-c-divider)) 14%, transparent);
  transition    : background var(--prose-transition), opacity var(--prose-transition),
                  box-shadow var(--prose-transition);
}

.tags-tag[data-on='false'] {
  background : color-mix(in srgb, var(--vp-c-text-3) 12%, transparent);
  opacity    : 0.75;
}

.tags-tag[data-active='true'] {
  z-index    : 5;
  box-shadow : 0 0 0 1px color-mix(in srgb, var(--family-color, var(--vp-c-brand-1)) 45%, transparent);
}

.tags-toggle {
  display     : inline-flex;
  align-items : center;
  padding     : 2px 7px;
  border      : 0;
  background  : transparent;
  cursor      : pointer;
}

.tags-slug {
  color          : var(--family-color, var(--vp-c-text-1));
  font-family    : var(--vp-font-family-mono);
  font-size      : 0.68rem;
  letter-spacing : 0.05em;
  text-transform : uppercase;
}

.tags-tag[data-on='false'] .tags-slug {
  color : var(--vp-c-text-3);
}

.tags-gear {
  display     : inline-flex;
  align-items : center;
  padding     : 2px 6px 2px 3px;
  border      : 0;
  background  : transparent;
  color       : var(--vp-c-text-3);
  cursor      : pointer;
  transition  : color var(--prose-transition), opacity var(--prose-transition);
}

.tags-gear:not(:disabled):hover,
.tags-gear[aria-expanded='true'] {
  color : var(--family-color, var(--vp-c-brand-1));
}

.tags-gear:disabled {
  opacity : 0.4;
  cursor  : default;
}

.tags-gear-icon {
  --glyph-size : 15px;
}

.tags-power {
  display       : grid;
  place-items   : center;
  align-self    : stretch;
  width         : 26px;
  padding       : 0;
  border        : 0;
  border-radius : var(--prose-radius-sm);
  background    : color-mix(in srgb, var(--vp-c-text-3) 12%, transparent);
  color         : var(--vp-c-text-2);
  cursor        : pointer;
  transition    : background var(--prose-transition), color var(--prose-transition);
}

.tags-power:hover {
  background : color-mix(in srgb, var(--vp-c-text-3) 20%, transparent);
  color      : var(--vp-c-text-1);
}

.tags-power[aria-pressed='true'] {
  color : var(--vp-c-text-3);
}

.tags-power:focus-visible {
  outline        : var(--prose-focus-ring);
  outline-offset : 1px;
}

.tags-power-icon {
  --glyph-size : 13px;
}

.tags-banner {
  display       : flex;
  min-height    : 5.5rem;
  max-height    : min(45vh, 30rem);
  padding       : 0.9rem 1.25rem;
  overflow-y    : auto;
  border        : 1px solid var(--vp-c-divider);
  border-radius : var(--prose-radius);
  background    : var(--vp-c-bg-soft);
}

.tags-banner :deep(.rule-card) {
  width      : 100%;
  min-height : auto;
}

.tags-idle {
  position : relative;
  display  : flex;
  width    : 100%;
}

.tags-idle-ghost {
  visibility : hidden;
}

.tags-idle .tags-hint {
  position    : absolute;
  inset       : 0;
  display     : grid;
  place-items : center;
  margin      : 0;
}

.tags-hint {
  margin     : auto;
  color      : var(--vp-c-text-3);
  font-size  : var(--prose-text-sm);
  text-align : center;
}
</style>
