<script setup lang="ts">
import { computed, ref } from 'vue'

import { data as primitives } from '../../../data/primitives-composition.data'

import type { PrimitiveLayer, PrimitiveSlug } from '../../../lib/shared/registries'

import PrimitivesCompositionDetail from './PrimitivesCompositionDetail.vue'
import PrimitivesCompositionGrid   from './PrimitivesCompositionGrid.vue'

const props = defineProps<{
  initialFocus ?: PrimitiveSlug
}>()

const focused = ref<PrimitiveSlug | null>(props.initialFocus ?? null)

const LAYER_NUMERAL: Record<PrimitiveLayer, string> = { analysis: 'III', base: 'I', orchestration: 'II' }

const bands = computed(() => [
  { entries : primitives.byLayer.analysis,      key : 'analysis'      as const, numeral : LAYER_NUMERAL.analysis      },
  { entries : primitives.byLayer.orchestration, key : 'orchestration' as const, numeral : LAYER_NUMERAL.orchestration },
  { entries : primitives.byLayer.base,          key : 'base'          as const, numeral : LAYER_NUMERAL.base          }
])

const focusedEntry = computed(() => focused.value === null ? null : primitives.entries.find(e => e.slug === focused.value) ?? null)

const related = computed<Set<string>>(() => {
  const s = new Set<string>()
  const f = focusedEntry.value
  if (!f) return s
  for (const dep   of f.consumes  ) s.add(dep  )
  for (const child of f.consumedBy) s.add(child)
  return s
})

function setFocus(slug: PrimitiveSlug) {
  focused.value = slug
}

function clearFocus() {
  if (props.initialFocus !== undefined) focused.value = props.initialFocus
}
</script>

<template>
  <div class="primitives-composition" :data-layer="focusedEntry?.layer ?? 'empty'">
    <PrimitivesCompositionGrid
      :bands   = "bands"
      :focused = "focused"
      :related = "related"
      @focus   = "setFocus"
      @blur    = "clearFocus"
    />
    <PrimitivesCompositionDetail v-if="initialFocus === undefined" :focused="focused" />
  </div>
</template>
