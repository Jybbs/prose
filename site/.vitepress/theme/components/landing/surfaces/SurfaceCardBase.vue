<script setup lang="ts">
import { useElementHover, useMouseInElement } from '@vueuse/core'
import { computed, ref }                      from 'vue'

import type { RenderedRule }            from '../../../../data/rules.data'
import { FAMILY_META, type RuleFamily } from '../../../../lib/shared/registries'

const props = defineProps<{
  bodyHtml : string
  family   : RuleFamily
  icon     : string
  number   : string
  rules    : readonly RenderedRule[]
}>()

const meta     = computed(() => FAMILY_META[props.family])
const category = computed(() => props.family === 'lint' ? 'lint' : 'auto-fix')
const href     = computed(() => `/rules/${props.family}/`)

const rootRef  = ref<HTMLElement | null>(null)
const chipsRef = ref<HTMLElement | null>(null)

const active = useElementHover(rootRef)

const { elementX: rx, elementY: ry, elementWidth: rw, elementHeight: rh } = useMouseInElement(rootRef)
const { elementX: cx, elementY: cy, elementWidth: cw, elementHeight: ch } = useMouseInElement(chipsRef)

const spotlightX     = computed(() => rw.value ? (rx.value / rw.value) * 100 : 50)
const spotlightY     = computed(() => rh.value ? (ry.value / rh.value) * 100 : 50)
const chipSpotlightX = computed(() => cw.value ? Math.max(0, Math.min(100, (cx.value / cw.value) * 100)) : 50)
const chipSpotlightY = computed(() => ch.value ? Math.max(0, Math.min(100, (cy.value / ch.value) * 100)) : 50)
</script>

<template>
  <div
    ref="rootRef"
    class="surface-card"
    :data-family="family"
    :data-category="category"
    :data-active="active"
    :style="{
      '--chip-spotlight-x' : `${chipSpotlightX}%`,
      '--chip-spotlight-y' : `${chipSpotlightY}%`,
      '--spotlight-x'      : `${spotlightX}%`,
      '--spotlight-y'      : `${spotlightY}%`
    }"
  >
    <a
      class="surface-card-cover-link"
      :href="href"
      :aria-label="`See all ${meta.label.toLowerCase()} rules`"
    />
    <span class="surface-card-number">— {{ number }}</span>
    <span class="surface-card-icon" aria-hidden="true">{{ icon }}</span>
    <h3 class="surface-card-label">{{ meta.label }}</h3>
    <p class="surface-card-blurb" v-html="bodyHtml" />
    <div ref="chipsRef" class="surface-card-chips">
      <slot :rules="rules" :active="active" />
    </div>
  </div>
</template>
