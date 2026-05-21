<script setup lang="ts">
import { computed, ref } from 'vue'

import type { RenderedRule }              from '../../../../data/rules.data'
import { FAMILY_META, type RuleFamily }   from '../../../../lib/shared/registries'

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

const active         = ref(false)
const chipSpotlightX = ref(50)
const chipSpotlightY = ref(50)
const chipsRef       = ref<HTMLElement | null>(null)
const spotlightX     = ref(50)
const spotlightY     = ref(50)

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
    :data-family="family"
    :data-category="category"
    :data-active="active"
    :style="{
      '--chip-spotlight-x' : `${chipSpotlightX}%`,
      '--chip-spotlight-y' : `${chipSpotlightY}%`,
      '--spotlight-x'      : `${spotlightX}%`,
      '--spotlight-y'      : `${spotlightY}%`
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
