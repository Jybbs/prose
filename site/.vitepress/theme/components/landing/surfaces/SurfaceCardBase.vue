<script setup lang="ts">
import { useElementHover, useElementSize, useMouseInElement } from '@vueuse/core'
import { computed, useTemplateRef }                           from 'vue'

import { provideAriaHidden }  from '../../../../lib/composables/use-aria-hidden'
import type { InlineNode }    from '../../../../lib/markdown/inline-nodes'
import type { RenderedRule }  from '../../../../lib/rules/rules.data'
import * as registries        from '../../../../lib/shared/registries'
import InlineProse            from '../../base/InlineProse.vue'
import SurfaceRail            from './SurfaceRail.vue'

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

const category = computed(() => registries.categoryOf(props.family))
const href     = computed(() => `/rules/${props.family}/`)
const meta     = computed(() => registries.FAMILY_META[props.family])
const tabindex = computed(() => (props.duplicate ? -1 : undefined))

const rootRef = useTemplateRef<HTMLElement>('root')

const active = useElementHover(rootRef)

const { elementX: rx, elementY: ry } = useMouseInElement(rootRef)
const { width: rw, height: rh }      = useElementSize(rootRef)

const spotlightX = computed(() => rw.value ? (rx.value / rw.value) * SPOTLIGHT_PCT_SCALE : SPOTLIGHT_FALLBACK_PCT)
const spotlightY = computed(() => rh.value ? (ry.value / rh.value) * SPOTLIGHT_PCT_SCALE : SPOTLIGHT_FALLBACK_PCT)
</script>

<template>
  <div
    ref="root"
    class="surface-card panel"
    :aria-hidden="duplicate || undefined"
    :data-family="family"
    :data-category="category"
    :data-active="active"
    :style="{
      '--spotlight-x' : `${spotlightX}%`,
      '--spotlight-y' : `${spotlightY}%`
    }"
  >
    <a
      class="surface-card-cover-link"
      :href="href"
      :aria-label="`See all ${meta.label.toLowerCase()} rules`"
      :tabindex="tabindex"
    />
    <span class="surface-card-number">— {{ number }}</span>
    <span class="surface-card-icon" aria-hidden="true">{{ meta.badge }}</span>
    <h3 class="surface-card-label">{{ meta.label }}</h3>
    <p class="surface-card-blurb"><InlineProse :nodes="bodyNodes" /></p>
    <div class="surface-card-chips">
      <SurfaceRail :rules="rules" />
    </div>
  </div>
</template>
