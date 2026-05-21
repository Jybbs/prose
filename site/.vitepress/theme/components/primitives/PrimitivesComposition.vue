<script setup lang="ts">
import { computed, ref } from 'vue'

import type { PrimitiveSlug }                                from '../../../lib/shared/registries'
import { PRIMITIVES }                                        from '../../../lib/shared/registries'
import { ENTRIES_BY_LAYER, LAYER_META, PRIMITIVE_ENTRIES }   from './primitives-composition-data'

const props = defineProps<{
  initialFocus ?: PrimitiveSlug
}>()

const hovered = ref<PrimitiveSlug | null>(props.initialFocus ?? null)

const layers = computed(() => [
  { entries: ENTRIES_BY_LAYER.analysis,      kicker: LAYER_META.analysis.kicker,      label: LAYER_META.analysis.label      },
  { entries: ENTRIES_BY_LAYER.orchestration, kicker: LAYER_META.orchestration.kicker, label: LAYER_META.orchestration.label },
  { entries: ENTRIES_BY_LAYER.base,          kicker: LAYER_META.base.kicker,          label: LAYER_META.base.label          }
])

const consumesByHovered = computed<Set<string>>(() => {
  if (!hovered.value) return new Set()
  const entry = PRIMITIVE_ENTRIES.find(e => e.slug === hovered.value)
  return new Set(entry?.consumes ?? [])
})

function isPrimitive(s: string): s is PrimitiveSlug {
  return s in PRIMITIVES
}

function isLit(slug: string): boolean {
  if (!hovered.value) return false
  if (slug === hovered.value) return true
  return consumesByHovered.value.has(slug)
}

function isDim(slug: string): boolean {
  return hovered.value !== null && !isLit(slug)
}
</script>

<template>
  <div class="primitives-composition">
    <div v-for="layer in layers" :key="layer.label" class="primitives-composition-row">
      <div class="primitives-composition-row-label">
        <span class="primitives-composition-row-kicker">{{ layer.kicker }}</span>
        <span class="primitives-composition-row-name">{{ layer.label }}</span>
      </div>
      <ul class="primitives-composition-row-cells">
        <li
          v-for="entry in layer.entries"
          :key="entry.slug"
          class="primitives-composition-cell"
          :class="{
            'primitives-composition-cell-lit': isLit(entry.slug),
            'primitives-composition-cell-dim': isDim(entry.slug)
          }"
          @mouseenter="hovered = entry.slug"
          @mouseleave="hovered = props.initialFocus ?? null"
          @focusin="hovered = entry.slug"
          @focusout="hovered = props.initialFocus ?? null"
        >
          <a class="primitives-composition-cell-link" :href="`/primitives/${entry.slug}`">
            <span class="primitives-composition-cell-name">{{ PRIMITIVES[entry.slug] }}</span>
            <span class="primitives-composition-cell-tagline">{{ entry.tagline }}</span>
          </a>
          <ul v-if="entry.consumes.length > 0" class="primitives-composition-cell-consumes" aria-label="Consumes">
            <li v-for="dep in entry.consumes" :key="dep">
              <template v-if="isPrimitive(dep)">{{ PRIMITIVES[dep] }}</template>
              <template v-else>{{ dep }}</template>
            </li>
          </ul>
        </li>
      </ul>
    </div>
    <p class="primitives-composition-legend">
      Hover or focus a primitive to highlight the row below it that the primitive consumes. Lower rows underpin higher ones.
    </p>
  </div>
</template>
