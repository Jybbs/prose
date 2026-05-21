<script setup lang="ts">
import { computed, ref } from 'vue'

import type { RenderedRule }              from '../../../../data/rules.data'
import { DOMAIN_META, type RuleDomain }   from '../../../../lib/shared/registries'

const props = defineProps<{
  bodyHtml : string
  domain   : RuleDomain
  icon     : string
  number   : string
  rules    : readonly RenderedRule[]
}>()

const meta     = computed(() => DOMAIN_META[props.domain])
const category = computed(() => props.domain === 'lint' ? 'lint' : 'auto-fix')
const href     = computed(() => `/rules/${props.domain}/`)

const chipsRef       = ref<HTMLElement | null>(null)
const spotlightX     = ref(50)
const spotlightY     = ref(50)
const chipSpotlightX = ref(50)
const chipSpotlightY = ref(50)
const active         = ref(false)

function clamp(value: number, lo: number, hi: number): number {
  return Math.max(lo, Math.min(hi, value))
}

function onPointerMove(event: PointerEvent) {
  const el   = event.currentTarget as HTMLElement
  const rect = el.getBoundingClientRect()
  spotlightX.value = ((event.clientX - rect.left) / rect.width)  * 100
  spotlightY.value = ((event.clientY - rect.top)  / rect.height) * 100

  if (chipsRef.value !== null) {
    const cr = chipsRef.value.getBoundingClientRect()
    chipSpotlightX.value = clamp(((event.clientX - cr.left) / cr.width)  * 100, 0, 100)
    chipSpotlightY.value = clamp(((event.clientY - cr.top)  / cr.height) * 100, 0, 100)
  }
}

function onEnter() { active.value = true  }
function onLeave() { active.value = false }
</script>

<template>
  <div
    class="surface-card"
    :data-domain="domain"
    :data-category="category"
    :data-active="active"
    :style="{
      '--spotlight-x'      : `${spotlightX}%`,
      '--spotlight-y'      : `${spotlightY}%`,
      '--chip-spotlight-x' : `${chipSpotlightX}%`,
      '--chip-spotlight-y' : `${chipSpotlightY}%`
    }"
    @pointermove="onPointerMove"
    @pointerenter="onEnter"
    @pointerleave="onLeave"
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
