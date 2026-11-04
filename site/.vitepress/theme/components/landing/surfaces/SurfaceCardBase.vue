<script setup lang="ts">
import { useElementHover }               from '@vueuse/core'
import { computed, useTemplateRef }      from 'vue'

import { provideAriaHidden } from '../../../../lib/composables/use-aria-hidden'
import type { InlineNode }   from '../../../../lib/markdown/inline-nodes'
import type { RenderedRule } from '../../../../lib/rules/rules.data'
import { familyEmblem }      from '../../../../lib/shared/emblems'
import * as registries       from '../../../../lib/shared/registries'
import InlineProse           from '../../base/InlineProse.vue'
import NibBadge              from '../../base/NibBadge.vue'
import SurfaceRail           from './SurfaceRail.vue'

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
</script>

<template>
  <div
    ref="root"
    class="surface-card relative isolate min-h-320 min-w-0 flex-col panel bg-bg-soft p-28 text-text-1 transition-colors duration-slow focus-within:border-family-edge-soft data-[active=true]:border-family-edge-soft motion-reduce:transition-none"
    :aria-hidden="duplicate || undefined"
    :data-family="family"
    :data-active="active"
  >
    <a
      class="surface-card-cover-link absolute inset-0 z-0 overflow-hidden rounded-base whitespace-nowrap"
      :href="href"
      :aria-label="`See all ${meta.label.toLowerCase()} rules`"
      :tabindex="tabindex"
    />
    <span class="pointer-events-none relative z-1 font-mono text-label tracking-label text-text-3">— {{ number }}</span>
    <span class="surface-card-icon pointer-events-none absolute top-22 right-22 z-2 flex size-56 items-center justify-center"><NibBadge :emblem="familyEmblem(family)" :size="36" /></span>
    <h3 class="pointer-events-none relative z-1 mt-42 mb-12 border-none p-0 font-display text-2rem leading-1-05 font-500 text-balance text-text-1">{{ meta.label }}</h3>
    <p class="surface-card-blurb pointer-events-none relative z-1 m-0 font-base text-md leading-1-6 text-text-2"><InlineProse :nodes="bodyNodes" /></p>
    <SurfaceRail class="pointer-events-none z-1" :rules="rules" />
  </div>
</template>
