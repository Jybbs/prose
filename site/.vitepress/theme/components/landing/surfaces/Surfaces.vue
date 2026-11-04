<script setup lang="ts">
import { computed, useTemplateRef } from 'vue'

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

const trackStyle = computed(() => ({
  transform: `translate3d(${-offset.value}px, 0, 0)`
}))
</script>

<template>
  <LandingSection
    centered
    :heading="heading"
    kicker="The Surfaces"
  >
    <template #heading-extra>
      <a href="/rules/" class="mt-18 inline-flex items-center rounded-sm border-1 border-divider bg-bg-soft px-12 py-4 font-mono text-kicker tracking-kicker text-text-2 uppercase transition-colors hover:border-brand-1 hover:bg-bg hover:text-brand-1 focus-visible:border-brand-1 focus-visible:bg-bg focus-visible:text-brand-1">All rules</a>
    </template>
    <div
      ref="viewport"
      class="surfaces-carousel group/carousel relative mt-8 w-screen overflow-hidden pt-32 pb-8 mask-x-from-95% data-fits:mask-none"
      :data-fits="fits || undefined"
      aria-label="Rule family carousel"
      @pointermove="onPointerMove"
      @pointerleave="onPointerLeave"
    >
      <div
        ref="track"
        class="flex w-max items-stretch gap-18 px-32 will-change-transform *:flex-none *:basis-360 group-data-fits/carousel:w-full group-data-fits/carousel:justify-center max-desktop:gap-14 max-desktop:*:basis-300 max-mobile:gap-12 max-mobile:*:basis-260"
        :style="trackStyle"
      >
        <template v-for="copy in 2" :key="copy">
          <SurfaceCardBase
            v-for="card in surfaceCards"
            :key="`${copy}-${card.family}`"
            :class="fits && copy === 2 ? 'hidden' : 'flex'"
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
