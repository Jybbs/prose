<script setup lang="ts">
import { computed, nextTick, ref, watch } from 'vue'

import { data as primitives } from '../../../lib/architecture/primitives-composition.data'
import { data as rules }      from '../../../lib/rules/rules.data'
import type { RenderedRule }  from '../../../lib/rules/rules.data'
import { useSettledMeasure }  from '../../../lib/composables/use-settled-measure'

import { familyEmblem }                              from '../../../lib/shared/emblems'
import { pickOr }                                    from '../../../lib/shared/pick-or'
import { PRIMITIVE_LAYER_NUMERALS, PRIMITIVE_SLUGS } from '../../../lib/shared/registries'
import type { PrimitiveSlug }                        from '../../../lib/shared/registries'
import { primitiveRoute }                            from '../../../lib/shared/routes'
import InlineProse                                   from '../base/InlineProse.vue'
import NibBadge                                      from '../base/NibBadge.vue'
import RuleTooltipPopper                             from '../rules/RuleTooltipPopper.vue'

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
    { items : entry.consumes,   keyPrefix : 'c', label : 'consumes',    port : 'in'  },
    { items : entry.consumedBy, keyPrefix : 'b', label : 'consumed by', port : 'out' }
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
  <div ref="cardRef" class="mt-14 flex flex-col gap-14 rounded-sm border-1 border-divider bg-bg px-22 py-18" :data-layer="focusedEntry?.layer ?? 'empty'" aria-live="polite">
    <template v-if="focusedEntry">
      <div class="flex items-center gap-14 border-b-1 border-b-divider pb-12 text-left">
        <span class="shrink-0 font-display text-1-5rem leading-none font-500 text-layer-ink italic" aria-hidden="true">{{ PRIMITIVE_LAYER_NUMERALS[focusedEntry.layer] }}</span>
        <div class="flex min-w-0 flex-1 flex-col gap-3">
          <span class="font-display text-1rem leading-1-1 text-text-1">{{ focusedEntry.name }}</span>
          <span class="primitives-composition-card-summary font-mono text-0-68rem leading-1-5 text-text-2"><InlineProse :nodes="focusedEntry.summaryNodes" /></span>
        </div>
      </div>
      <template v-for="rel in relations" :key="rel.label">
        <div v-if="rel.items.length > 0" class="grid grid-cols-110-minmax-1fr items-center gap-x-10">
          <span class="text-left font-mono text-0-54rem tracking-kicker whitespace-nowrap text-text-2 uppercase">
            <span class="mr-6 inline-block port-tab bg-text-2" aria-hidden="true">{{ rel.port }}</span>
            {{ rel.label }}
          </span>
          <span class="flex min-h-24 flex-wrap items-center gap-x-16 gap-y-5 font-mono text-0-68rem text-text-2">
            <span v-for="m in rel.items" :key="`${rel.keyPrefix}-${m.slug}`" class="primitives-composition-card-mention-item relative inline-flex items-center not-last:after:pointer-events-none not-last:after:absolute not-last:after:top-1/2 not-last:after:left-full not-last:after:ml-8 not-last:after:-translate-1/2 not-last:after:text-text-3 not-last:after:content-middot data-suppress-dot:after:hidden">
              <a v-if="m.kind === 'primitive'" class="inline-flex items-center gap-5 text-0-68rem text-text-1 no-underline" :data-layer="m.layer" :href="primitiveRoute(m.slug)">
                <span class="font-display text-xs leading-none font-500 text-layer-ink italic" :data-layer="m.layer" aria-hidden="true">{{ m.numeral }}</span>
                <span class="nib-stroke pb-2">{{ m.slug }}</span>
              </a>
              <RuleTooltipPopper v-else-if="m.kind === 'rule'" :rule="m.rule">
                <a class="chip nib-ground gap-6 py-2 pr-8 pl-6 text-0-68rem leading-1-5 text-text-1" :href="m.rule.href" :data-family="m.rule.family">
                  <NibBadge :emblem="familyEmblem(m.rule.family)" :size="12.5" />
                  <span class="font-mono text-0-68rem text-text-1">{{ m.slug }}</span>
                </a>
              </RuleTooltipPopper>
              <span v-else class="text-0-68rem text-text-3">{{ m.slug }}</span>
            </span>
          </span>
        </div>
      </template>
    </template>
    <p v-else class="text-sm text-text-3 italic">Hover a tile to see what it draws from and what it feeds into.</p>
  </div>
</template>
