<script setup lang="ts">
import { computed, useTemplateRef, watchEffect } from 'vue'

import { useCarouselMeasurement } from '../../../../lib/composables/use-carousel-measurement'
import { useCarouselVelocity }    from '../../../../lib/composables/use-carousel-velocity'
import { useReducedMotion }       from '../../../../lib/composables/use-reduced-motion'
import { data as landing }        from '../../../../lib/landing/landing.data'
import { data as rules }          from '../../../../lib/rules/rules.data'
import LandingSection             from '../LandingSection.vue'
import SurfaceCardBase            from './SurfaceCardBase.vue'

const BASE_SPEED_PX_PER_SEC = 32
const EDGE_MARGIN_PX        = 32
const MAGNET_GAIN           = 4
const MAX_PULL_PX_PER_SEC   = BASE_SPEED_PX_PER_SEC * 8

const surfaceCards = computed(() =>
  landing.surfaces.map(s => ({ ...s, rules: rules.byFamily[s.family] ?? [] }))
)

const ruleCount = computed(() => rules.list.length)

const heading = computed(() =>
  `<strong>${surfaceCards.value.length}</strong> rule families. <em>${ruleCount.value}</em> rules.`
)

const trackRef      = useTemplateRef<HTMLElement>('track')
const viewportRef   = useTemplateRef<HTMLElement>('viewport')
const reducedMotion = useReducedMotion()

const { fits, halfWidth } = useCarouselMeasurement(trackRef, viewportRef, () => surfaceCards.value.length)
const { offset, onPointerLeave, onPointerMove } = useCarouselVelocity(viewportRef, halfWidth, fits, {
  baseSpeedPxPerSec : BASE_SPEED_PX_PER_SEC,
  edgeMarginPx      : EDGE_MARGIN_PX,
  magnetGain        : MAGNET_GAIN,
  maxPullPxPerSec   : MAX_PULL_PX_PER_SEC,
  reducedMotion
})

watchEffect(() => {
  if (fits.value) {
    offset.value = 0
  }
  else if (halfWidth.value > 0) {
    offset.value = ((offset.value % halfWidth.value) + halfWidth.value) % halfWidth.value
  }
})

const trackStyle = computed(() => ({
  transform: `translate3d(${-offset.value}px, 0, 0)`
}))
</script>

<template>
  <LandingSection
    centered
    :heading="heading"
    kicker="The Surfaces"
    variant="surfaces"
  >
    <template #heading-extra>
      <a href="/rules/" class="landing-small-button">All rules</a>
    </template>
    <div
      ref="viewport"
      class="surfaces-carousel"
      :class="{ 'surfaces-carousel-static': fits }"
      aria-label="Rule family carousel"
      @pointermove="onPointerMove"
      @pointerleave="onPointerLeave"
    >
      <div
        ref="track"
        class="surfaces-carousel-track"
        :style="trackStyle"
      >
        <template v-for="copy in 2" :key="copy">
          <SurfaceCardBase
            v-for="card in surfaceCards"
            :key="`${copy}-${card.family}`"
            :body-nodes="card.bodyNodes"
            :duplicate="copy === 2"
            :family="card.family"
            :number="card.number"
            :rules="card.rules"
          />
        </template>
      </div>
    </div>
  </LandingSection>
</template>
