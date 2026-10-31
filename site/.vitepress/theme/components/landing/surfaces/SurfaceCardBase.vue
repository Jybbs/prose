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
    class="surface-card panel"
    :aria-hidden="duplicate || undefined"
    :data-family="family"
    :data-active="active"
  >
    <a
      class="surface-card-cover-link"
      :href="href"
      :aria-label="`See all ${meta.label.toLowerCase()} rules`"
      :tabindex="tabindex"
    />
    <span class="surface-card-number">— {{ number }}</span>
    <span class="surface-card-icon"><NibBadge :emblem="familyEmblem(family)" :size="36" /></span>
    <h3 class="surface-card-label">{{ meta.label }}</h3>
    <p class="surface-card-blurb"><InlineProse :nodes="bodyNodes" /></p>
    <SurfaceRail :rules="rules" />
  </div>
</template>
