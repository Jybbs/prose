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
    class="plate"
    :data-family="rule.family || null"
    :aria-label="`Settings for ${rule.slug}`"
  >
    <div class="plate-specimen">
      <RuleCard v-if="card" :rule="card" :clickable="false" />
      <div v-else class="plate-stub">
        <span class="plate-stub-slug">{{ rule.slug }}</span>
      </div>
      <button
        type="button"
        class="plate-seat plate-close"
        aria-label="Close"
        @click="$emit('close')"
      >
        <svg class="glyph" viewBox="0 0 24 24" fill="none" aria-hidden="true">
          <path d="M6 6l12 12" />
          <path d="M18 6L6 18" />
        </svg>
      </button>
    </div>
    <div class="plate-divider" aria-hidden="true" />
    <div class="plate-rows">
      <div v-for="facet in facets" :key="facet.key" class="plate-row">
        <span class="plate-label">{{ facet.label }}</span>
        <span class="plate-hint" v-html="facet.hintHtml" />
        <button
          v-if="facet.kind === 'bool'"
          type="button"
          role="switch"
          class="plate-control plate-switch"
          :aria-checked="boolValue(facet)"
          :aria-label="facet.label"
          @click="sandbox.setFacet(rule.slug, facet, !boolValue(facet))"
        >
          <span class="plate-switch-knob" />
        </button>
        <input
          v-else-if="facet.kind === 'int'"
          type="number"
          class="plate-control plate-number"
          :value="numberValue(facet)"
          :aria-label="facet.label"
          @input="sandbox.setFacet(rule.slug, facet, Number(($event.target as HTMLInputElement).value))"
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
