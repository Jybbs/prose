<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'

import { data as primitives }    from '../../../data/primitives-composition.data'
import { data as primitiveMeta } from '../../../data/primitives.data'
import { data as rules }         from '../../../data/rules.data'
import { useSettledMeasure }     from '../../../lib/composables/use-settled-measure'

import { PRIMITIVE_LAYER_NUMERALS }           from '../../../lib/shared/registries'
import type { PrimitiveLayer, PrimitiveSlug } from '../../../lib/shared/registries'

const props = defineProps<{
  focused : PrimitiveSlug | null
}>()

const focusedEntry = computed(() => props.focused === null ? null : primitives.bySlug[props.focused] ?? null)

const relations = computed(() => {
  const f = focusedEntry.value
  if (!f) return []
  return [
    { items : f.consumes,   keyPrefix : 'c', label : 'consumes'    },
    { items : f.consumedBy, keyPrefix : 'b', label : 'consumed by' }
  ]
})

function isPrimitive(s: string): s is PrimitiveSlug {
  return s in primitiveMeta.bySlug
}

function layerOf(slug: string): string {
  return primitives.bySlug[slug]?.layer ?? 'empty'
}

function numeralOf(slug: string): string {
  const layer = layerOf(slug)
  return PRIMITIVE_LAYER_NUMERALS[layer as PrimitiveLayer] ?? ''
}

function ruleOf(slug: string) {
  return rules.bySlug[slug] ?? null
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
          <span class="primitives-composition-card-name">{{ primitiveMeta.bySlug[focusedEntry.slug].name }}</span>
          <span class="primitives-composition-card-summary" v-html="focusedEntry.summaryHtml" />
        </div>
      </div>
      <template v-for="rel in relations" :key="rel.label">
        <div v-if="rel.items.length > 0" class="primitives-composition-card-rel">
          <span class="primitives-composition-card-rel-label">{{ rel.label }}</span>
          <span class="primitives-composition-card-rel-mentions">
            <span v-for="dep in rel.items" :key="`${rel.keyPrefix}-${dep}`" class="primitives-composition-card-mention-item">
              <a v-if="isPrimitive(dep)" class="primitives-composition-card-mention" :data-layer="layerOf(dep)" :href="`/primitives/${dep}`">
                <span class="primitives-composition-card-mention-chip" :data-layer="layerOf(dep)" aria-hidden="true">{{ numeralOf(dep) }}</span>
                <span class="primitives-composition-card-mention-text">{{ dep }}</span>
              </a>
              <RuleTooltipPopper v-else-if="ruleOf(dep)" :rule="ruleOf(dep)!">
                <a class="rule-chip" :href="ruleOf(dep)!.href" :data-family="ruleOf(dep)!.family">
                  <span class="rule-chip-badge" aria-hidden="true">{{ ruleOf(dep)!.familyBadge }}</span>
                  <span class="rule-chip-slug">{{ dep }}</span>
                </a>
              </RuleTooltipPopper>
              <span v-else class="primitives-composition-card-mention-ext">{{ dep }}</span>
            </span>
          </span>
        </div>
      </template>
    </template>
    <p v-else class="primitives-composition-card-hint">Hover a tile to see what it draws from and what it feeds into.</p>
  </div>
</template>
