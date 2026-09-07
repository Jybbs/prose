<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'

import { data as primitives } from '../../../lib/primitives/primitives-composition.data'
import { data as rules }      from '../../../lib/rules/rules.data'
import type { RenderedRule }  from '../../../lib/rules/rules.data'
import { useSettledMeasure }  from '../../../lib/composables/use-settled-measure'

import { pickOr }                                    from '../../../lib/shared/pick-or'
import { PRIMITIVE_LAYER_NUMERALS, PRIMITIVE_SLUGS } from '../../../lib/shared/registries'
import type { PrimitiveSlug }                        from '../../../lib/shared/registries'
import InlineProse                                   from '../base/InlineProse.vue'

const props = defineProps<{
  focused : PrimitiveSlug | null
}>()

const focusedEntry = computed(() =>
  props.focused === null ? null : pickOr(primitives.bySlug, props.focused, null))

type Mention =
  | { kind : 'primitive', layer : string, numeral : string, slug : string }
  | { kind : 'rule',      rule  : RenderedRule,             slug : string }
  | { kind : 'external',                                    slug : string }

const relations = computed(() => {
  const entry = focusedEntry.value
  if (!entry) return []
  return [
    { items : entry.consumes,   keyPrefix : 'c', label : 'consumes'    },
    { items : entry.consumedBy, keyPrefix : 'b', label : 'consumed by' }
  ].map(rel => ({ ...rel, items: rel.items.map(mentionOf) }))
})

function mentionOf(slug: string): Mention {
  if (isPrimitive(slug)) {
    return { kind: 'primitive', layer: layerOf(slug), numeral: numeralOf(slug), slug }
  }
  const rule = ruleOf(slug)
  return rule === null ? { kind: 'external', slug } : { kind: 'rule', rule, slug }
}

function isPrimitive(s: string): s is PrimitiveSlug {
  return (PRIMITIVE_SLUGS as readonly string[]).includes(s)
}

function layerOf(slug: string): string {
  return primitives.bySlug[slug]?.layer ?? 'empty'
}

function numeralOf(slug: string): string {
  return pickOr(PRIMITIVE_LAYER_NUMERALS, layerOf(slug), '')
}

function ruleOf(slug: string) {
  return pickOr(rules.bySlug, slug, null)
}

const cardRef = ref<HTMLElement | null>(null)

function updateWrapMarkers() {
  const root = cardRef.value
  if (!root) return
  for (const row of root.querySelectorAll<HTMLElement>('.primitives-composition-card-rel-mentions')) {
    const items = Array.from(row.querySelectorAll<HTMLElement>('.primitives-composition-card-mention-item'))
    for (let i = 0; i < items.length; i++) {
      const item = items[i]
      const next = items[i + 1]
      if (next && next.offsetTop > item.offsetTop) item.dataset.suppressDot = ''
      else                                         delete item.dataset.suppressDot
    }
  }
}

const scheduleUpdate = () => nextTick(updateWrapMarkers)
useSettledMeasure(cardRef, scheduleUpdate)
watch(focusedEntry, scheduleUpdate, { immediate: true })
</script>

<template>
  <div ref="cardRef" class="primitives-composition-card" :data-layer="focusedEntry?.layer ?? 'empty'" aria-live="polite">
    <template v-if="focusedEntry">
      <div class="primitives-composition-card-head">
        <span class="primitives-composition-card-layer-numeral" aria-hidden="true">{{ PRIMITIVE_LAYER_NUMERALS[focusedEntry.layer] }}</span>
        <div class="primitives-composition-card-head-text">
          <span class="primitives-composition-card-name">{{ focusedEntry.name }}</span>
          <span class="primitives-composition-card-summary"><InlineProse :nodes="focusedEntry.summaryNodes" /></span>
        </div>
      </div>
      <template v-for="rel in relations" :key="rel.label">
        <div v-if="rel.items.length > 0" class="primitives-composition-card-rel">
          <span class="primitives-composition-card-rel-label">{{ rel.label }}</span>
          <span class="primitives-composition-card-rel-mentions">
            <span v-for="m in rel.items" :key="`${rel.keyPrefix}-${m.slug}`" class="primitives-composition-card-mention-item">
              <a v-if="m.kind === 'primitive'" class="primitives-composition-card-mention" :data-layer="m.layer" :href="`/primitives/${m.slug}`">
                <span class="primitives-composition-card-mention-chip" :data-layer="m.layer" aria-hidden="true">{{ m.numeral }}</span>
                <span class="primitives-composition-card-mention-text">{{ m.slug }}</span>
              </a>
              <RuleTooltipPopper v-else-if="m.kind === 'rule'" :rule="m.rule">
                <a class="rule-chip" :href="m.rule.href" :data-family="m.rule.family">
                  <span class="rule-chip-badge" aria-hidden="true">{{ m.rule.familyBadge }}</span>
                  <span class="rule-chip-slug">{{ m.slug }}</span>
                </a>
              </RuleTooltipPopper>
              <span v-else class="primitives-composition-card-mention-ext">{{ m.slug }}</span>
            </span>
          </span>
        </div>
      </template>
    </template>
    <p v-else class="primitives-composition-card-hint">Hover a tile to see what it draws from and what it feeds into.</p>
  </div>
</template>
