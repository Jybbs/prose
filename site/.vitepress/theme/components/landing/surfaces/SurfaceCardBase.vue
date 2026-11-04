<script setup lang="ts">
import { useElementHover }               from '@vueuse/core'
import { computed, ref, useTemplateRef } from 'vue'

import { provideAriaHidden } from '../../../../lib/composables/use-aria-hidden'
import type { InlineNode }   from '../../../../lib/markdown/inline-nodes'
import type { RenderedRule } from '../../../../lib/rules/rules.data'
import { familyEmblem }      from '../../../../lib/shared/emblems'
import * as registries       from '../../../../lib/shared/registries'
import InlineProse           from '../../base/InlineProse.vue'
import NibBadge              from '../../base/NibBadge.vue'
import SurfaceRail           from './SurfaceRail.vue'

const SPOTLIGHT_FALLBACK_PCT = 50
const SPOTLIGHT_PCT_SCALE    = 100

const props = defineProps<{
  bodyNodes : InlineNode[]
  duplicate : boolean
  family    : registries.RuleFamily
  number    : string
  rules     : readonly RenderedRule[]
}>()

provideAriaHidden(() => props.duplicate)

const href     = computed(() => `/rules/${props.family}/`)
const meta     = computed(() => registries.FAMILY_META[props.family])
const tabindex = computed(() => (props.duplicate ? -1 : undefined))

const rootRef = useTemplateRef<HTMLElement>('root')

const active = useElementHover(rootRef)

const spotlightX = ref(SPOTLIGHT_FALLBACK_PCT)
const spotlightY = ref(SPOTLIGHT_FALLBACK_PCT)

function onPointerLeave() {
  spotlightX.value = SPOTLIGHT_FALLBACK_PCT
  spotlightY.value = SPOTLIGHT_FALLBACK_PCT
}

function onPointerMove(event: PointerEvent) {
  const rect       = (event.currentTarget as HTMLElement).getBoundingClientRect()
  spotlightX.value = (event.clientX - rect.left) / rect.width * SPOTLIGHT_PCT_SCALE
  spotlightY.value = (event.clientY - rect.top) / rect.height * SPOTLIGHT_PCT_SCALE
}
</script>

<template>
  <div
    ref="root"
    class="surface-card relative isolate min-h-320 min-w-0 flex-col panel bg-bg-soft p-28 text-text-1 transition-border-color-translate-and-box-shadow duration-slow before:absolute before:inset-x-0 before:top-0 before:z-2 before:h-3 before:rounded-t-base before:opacity-0 before:transition-opacity before:duration-slow after:pointer-events-none after:absolute after:inset-0 after:z-0 after:rounded-base focus-within:-translate-y-2 focus-within:border-family focus-within:before:opacity-100 data-active:-translate-y-2 data-active:border-family data-active:before:opacity-100"
    :aria-hidden="duplicate || undefined"
    :data-family="family"
    :data-active="active || undefined"
    :style="{
      '--spotlight-x' : `${spotlightX}%`,
      '--spotlight-y' : `${spotlightY}%`
    }"
    @pointermove="onPointerMove"
    @pointerleave="onPointerLeave"
  >
    <a
      class="surface-card-cover-link absolute inset-0 z-0 overflow-hidden rounded-base whitespace-nowrap"
      :href="href"
      :aria-label="`See all ${meta.label.toLowerCase()} rules`"
      :tabindex="tabindex"
    />
    <span class="pointer-events-none relative z-1 font-mono text-label tracking-label text-text-3">— {{ number }}</span>
    <span class="surface-card-icon pointer-events-none absolute top-22 right-22 z-2 medallion size-56 border-family-35-divider ground"><NibBadge :emblem="familyEmblem(family)" :size="24.8" /></span>
    <h3 class="pointer-events-none relative z-1 mt-42 mb-12 border-none font-display text-2rem leading-1-05 font-500 text-balance">{{ meta.label }}</h3>
    <p class="surface-card-blurb pointer-events-none relative z-1 text-md leading-1-6"><InlineProse :nodes="bodyNodes" /></p>
    <SurfaceRail class="pointer-events-none z-1" :rules="rules" />
  </div>
</template>
