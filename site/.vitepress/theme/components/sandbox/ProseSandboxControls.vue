<script setup lang="ts">
import { onKeyStroke }   from '@vueuse/core'
import { computed, ref } from 'vue'

import RuleCard     from '../rules/RuleCard.vue'
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

function boolValue(facet: chipPanel.Facet): boolean {
  return props.sandbox.facetValue(openSlug.value, facet) === true
}

function hover(rule: chipPanel.RuleControl): void {
  hoveredSlug.value = rule.slug
}

function numberValue(facet: chipPanel.Facet): number {
  return props.sandbox.facetValue(openSlug.value, facet) as number
}

function textValue(facet: chipPanel.Facet): string {
  const value = props.sandbox.facetValue(openSlug.value, facet)
  return Array.isArray(value) ? value.join(', ') : String(value)
}

function writeText(facet: chipPanel.Facet, raw: string): void {
  const value = facet.kind === 'stringList'
    ? raw.split(',').map(part => part.trim()).filter(Boolean)
    : raw
  props.sandbox.setFacet(openSlug.value, facet, value)
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
        <svg class="tags-power-icon" viewBox="0 0 24 24" fill="none" aria-hidden="true">
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
          <svg class="tags-gear-icon" viewBox="0 0 24 24" fill="none" aria-hidden="true">
            <path
              d="M10.3 4.3c.4-1.8 2.9-1.8 3.4 0a1.7 1.7 0 0 0 2.5 1.1c1.6-.9 3.3.8 2.4 2.4a1.7 1.7 0 0 0 1 2.5c1.8.4 1.8 2.9 0 3.4a1.7 1.7 0 0 0-1 2.5c.9 1.6-.8 3.3-2.4 2.4a1.7 1.7 0 0 0-2.5 1c-.5 1.8-3 1.8-3.4 0a1.7 1.7 0 0 0-2.5-1c-1.6.9-3.3-.8-2.4-2.4a1.7 1.7 0 0 0-1-2.5c-1.8-.5-1.8-3 0-3.4a1.7 1.7 0 0 0 1-2.5c-.9-1.6.8-3.3 2.4-2.4a1.7 1.7 0 0 0 2.5-1z"
            />
            <path d="M9 12a3 3 0 1 0 6 0a3 3 0 0 0-6 0" />
          </svg>
        </button>
      </div>
    </div>

    <aside class="tags-banner">
      <section
        v-if="openRule"
        :ref="setPanel"
        class="plate"
        :data-family="openRule.family || null"
        :aria-label="`Settings for ${openRule.slug}`"
      >
        <div class="plate-specimen">
          <RuleCard v-if="pinnedCard" :rule="pinnedCard" :clickable="false" />
          <div v-else class="plate-stub">
            <span class="plate-stub-slug">{{ openRule.slug }}</span>
          </div>
          <button
            type="button"
            class="plate-seat plate-close"
            aria-label="Close"
            @click="openSlug = ''"
          >
            <svg class="plate-seat-glyph" viewBox="0 0 24 24" fill="none" aria-hidden="true">
              <path d="M6 6l12 12" />
              <path d="M18 6L6 18" />
            </svg>
          </button>
        </div>
        <div class="plate-divider" aria-hidden="true" />
        <div class="plate-rows">
          <div v-for="facet in subFacets(openRule)" :key="facet.key" class="plate-row">
            <span class="plate-label">{{ facet.label }}</span>
            <span class="plate-hint" v-html="facet.hintHtml" />
            <button
              v-if="facet.kind === 'bool'"
              type="button"
              role="switch"
              class="plate-control plate-switch"
              :aria-checked="boolValue(facet)"
              :aria-label="facet.label"
              @click="sandbox.setFacet(openSlug, facet, !boolValue(facet))"
            >
              <span class="plate-switch-knob" />
            </button>
            <input
              v-else-if="facet.kind === 'int'"
              type="number"
              class="plate-control plate-number"
              :value="numberValue(facet)"
              :aria-label="facet.label"
              @input="sandbox.setFacet(openSlug, facet, Number(($event.target as HTMLInputElement).value))"
            >
            <input
              v-else
              type="text"
              class="plate-control plate-text"
              :value="textValue(facet)"
              :aria-label="facet.label"
              @change="writeText(facet, ($event.target as HTMLInputElement).value)"
            >
          </div>
        </div>
      </section>
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
@keyframes plate-draw {
  to {
    transform : scaleX(1);
  }
}

@keyframes plate-settle {
  from {
    opacity   : 0;
    translate : 0 -4px;
  }
}

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
  width          : 15px;
  height         : 15px;
  stroke         : currentColor;
  stroke-width   : 2;
  stroke-linecap : round;
}

.plate-seat {
  display         : inline-flex;
  flex            : none;
  align-items     : center;
  justify-content : center;
  width           : 26px;
  height          : 26px;
  padding         : 0;
  border          : 1px solid var(--vp-c-divider);
  border-radius   : var(--prose-radius-sm);
  background      : var(--vp-c-bg);
  color           : var(--vp-c-text-3);
  cursor          : pointer;
  transition      : color var(--prose-transition), border-color var(--prose-transition);
}

.plate-seat:focus-visible {
  outline        : var(--prose-focus-ring);
  outline-offset : 1px;
}

.plate-seat-glyph {
  width           : 14px;
  height          : 14px;
  stroke          : currentColor;
  stroke-width    : 2;
  stroke-linecap  : round;
  stroke-linejoin : round;
}

.plate {
  --prose-panel-wash : color-mix(in srgb, var(--vp-c-bg-alt) 35%, var(--vp-c-bg));
  --plate-accent     : var(--family-color, var(--vp-c-brand-1));
  --plate-control-h  : 22px;
  --plate-switch-w   : 40px;

  width         : 100%;
  min-width     : 0;
  border        : 1px solid color-mix(in srgb, var(--plate-accent) 40%, var(--vp-c-divider));
  border-radius : var(--prose-radius-sm);
  background    : color-mix(in srgb, var(--plate-accent) 4%, var(--prose-panel-wash));
}

.plate :deep(.rule-card) {
  border     : none;
  background : transparent;
}

.plate-specimen {
  position : relative;
}

.plate-stub {
  padding : 12px 16px;
}

.plate-stub-slug {
  color          : var(--plate-accent);
  font-family    : var(--vp-font-family-mono);
  font-size      : var(--prose-kicker-size-sm);
  letter-spacing : var(--prose-kicker-tracking);
  text-transform : uppercase;
}

/* The close seat's right edge sits on the 16px inset the rows below share, so
   it reads as the head of the right-aligned control rail. */
.plate-close {
  position  : absolute;
  top       : 50%;
  right     : 16px;
  transform : translateY(-50%);
  animation : plate-settle var(--prose-transition-slow);
}

.plate-close:hover {
  border-color : color-mix(in srgb, var(--plate-accent) 55%, var(--vp-c-divider));
  color        : var(--plate-accent);
}

.plate-divider {
  position         : relative;
  height           : 1px;
  margin           : 0 16px;
  background       : var(--vp-c-divider);
  transform        : scaleX(0);
  transform-origin : left;
  animation        : plate-draw calc(var(--prose-rule-draw-ms) * 1ms) ease forwards;
}

.plate-divider::after {
  content    : "";
  position   : absolute;
  top        : -1px;
  left       : 0;
  width      : 30px;
  height     : 2px;
  background : var(--plate-accent);
}

/* The 16px inset matches the card's own padding, so the label column opens at
   the badge's left edge and the table spans the full card measure. */
.plate-rows {
  display               : grid;
  grid-template-columns : max-content minmax(0, 1fr) max-content;
  column-gap            : 1.1rem;
  padding               : 0 16px 8px;
  animation             : plate-settle var(--prose-transition-slow);
}

.plate-row {
  display               : grid;
  grid-column           : 1 / -1;
  grid-template-columns : subgrid;
  align-items           : center;
  min-height            : 2.1rem;
}

.plate-row + .plate-row {
  border-top : 1px solid var(--vp-c-divider);
}

.plate-label {
  color          : var(--plate-accent);
  font-family    : var(--vp-font-family-mono);
  font-weight    : 500;
  font-size      : var(--prose-text-xxs);
  letter-spacing : 0.02em;
  text-transform : uppercase;
}

.plate-hint {
  padding-block : 0.25rem;
  color         : var(--vp-c-text-3);
  font-family   : var(--vp-font-family-base);
  font-style    : italic;
  font-size     : var(--prose-text-xs);
  line-height   : 1.4;
}

.plate-hint :deep(code) {
  padding       : 2px 6px;
  border-radius : var(--prose-radius-sm);
  background    : color-mix(in srgb, var(--vp-c-text-1) 8%, transparent);
  color         : var(--plate-accent);
  font-family   : var(--vp-font-family-mono);
  font-style    : normal;
  font-size     : 0.86em;
  white-space   : nowrap;
}

.plate-control {
  justify-self : end;
}

.plate-switch {
  position      : relative;
  width         : var(--plate-switch-w);
  height        : var(--plate-control-h);
  padding       : 0;
  border        : 0;
  border-radius : 999px;
  background    : color-mix(in srgb, var(--vp-c-text-3) 40%, transparent);
  cursor        : pointer;
  transition    : background var(--prose-transition);
}

.plate-switch[aria-checked='true'] {
  background : var(--plate-accent);
}

.plate-switch-knob {
  position      : absolute;
  top           : 3px;
  left          : 3px;
  width         : 16px;
  height        : 16px;
  border-radius : 50%;
  background    : var(--vp-c-bg);
  transition    : transform var(--prose-transition);
}

.plate-switch[aria-checked='true'] .plate-switch-knob {
  transform : translateX(18px);
}

.plate-number,
.plate-text {
  height        : var(--plate-control-h);
  border        : 1px solid color-mix(in srgb, var(--plate-accent) 55%, var(--vp-c-divider));
  border-radius : var(--prose-radius-sm);
  background    : var(--vp-c-bg);
  color         : var(--vp-c-text-1);
  font-family   : var(--vp-font-family-mono);
  font-size     : var(--prose-text-xs);
}

.plate-number {
  width                : var(--plate-switch-w);
  padding              : 0;
  font-variant-numeric : tabular-nums;
  line-height          : calc(var(--plate-control-h) - 2px);
  text-align           : center;
  appearance           : textfield;
}

.plate-number::-webkit-outer-spin-button,
.plate-number::-webkit-inner-spin-button {
  margin     : 0;
  appearance : none;
}

.plate-text {
  width   : 9rem;
  padding : 0 0.4rem;
}

.plate-switch:focus-visible,
.plate-number:focus-visible,
.plate-text:focus-visible {
  outline        : var(--prose-focus-ring);
  outline-offset : 1px;
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
  width          : 13px;
  height         : 13px;
  stroke         : currentColor;
  stroke-width   : 2;
  stroke-linecap : round;
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

@media (--prose-bp-medium) {
  .plate-rows {
    grid-template-columns : minmax(0, 1fr) max-content;
  }

  .plate-row {
    row-gap       : 0.1rem;
    padding-block : 0.4rem;
  }

  .plate-label {
    grid-column : 1;
    grid-row    : 1;
  }

  .plate-hint {
    grid-column   : 1;
    grid-row      : 2;
    padding-block : 0;
  }

  .plate-control {
    grid-column : 2;
    grid-row    : 1 / span 2;
  }
}

@media (prefers-reduced-motion: reduce) {
  .plate-close,
  .plate-rows {
    animation : none;
  }

  .plate-divider {
    animation : none;
    transform : scaleX(1);
  }

  .plate-switch-knob {
    transition : none;
  }
}
</style>
