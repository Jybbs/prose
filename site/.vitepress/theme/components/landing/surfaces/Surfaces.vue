<script setup lang="ts">
import { useElementBounding, useEventListener, useMediaQuery, useRafFn, useResizeObserver } from '@vueuse/core'
import { computed, onMounted, ref, useTemplateRef }                                         from 'vue'

import LandingSection from '../LandingSection.vue'

import { data as landing }     from '../../../../data/landing.data'
import { data as rules }       from '../../../../data/rules.data'

import SurfaceCardBase from './SurfaceCardBase.vue'

const surfaceCards = computed(() =>
  landing.surfaces.map(s => ({ ...s, rules: rules.byFamily[s.family] ?? [] }))
)

const ruleCount = computed(() => rules.list.length)

const heading = computed(() =>
  `<strong>${surfaceCards.value.length}</strong> rule families. <em>${ruleCount.value}</em> rules.`
)

const offset      = ref(0)
const trackRef    = useTemplateRef<HTMLElement>('track')
const viewportRef = useTemplateRef<HTMLElement>('viewport')

const BASE_SPEED_PX_PER_SEC = 32
const EDGE_MARGIN_PX        = 32
const MAGNET_GAIN           = 4
const MAX_PULL_PX_PER_SEC   = BASE_SPEED_PX_PER_SEC * 8

const reducedMotion                    = useMediaQuery('(prefers-reduced-motion: reduce)')
const { left: vpLeft, right: vpRight } = useElementBounding(viewportRef)

let halfWidth = 0
let velocity  = BASE_SPEED_PX_PER_SEC

function measure() {
  const track = trackRef.value
  if (!track) return
  const originalCount = surfaceCards.value.length
  const cards         = track.children
  if (cards.length >= originalCount + 1) {
    const firstCard    = cards[0]             as HTMLElement
    const firstDupCard = cards[originalCount] as HTMLElement
    halfWidth = firstDupCard.offsetLeft - firstCard.offsetLeft
  } else {
    halfWidth = 0
  }
  if (halfWidth > 0) {
    offset.value = ((offset.value % halfWidth) + halfWidth) % halfWidth
  }
}

function wrap(value: number): number {
  if (halfWidth <= 0) return value
  return ((value % halfWidth) + halfWidth) % halfWidth
}

useRafFn(({ delta }) => {
  if (halfWidth > 0 && !reducedMotion.value) {
    offset.value = wrap(offset.value + velocity * delta / 1000)
  }
}, { immediate: true })

function onPointerMove(event: PointerEvent) {
  const node = (event.target as HTMLElement).closest('.surface-card') as HTMLElement | null
  if (!node) {
    velocity = 0
    return
  }

  const cardRect = node.getBoundingClientRect()
  const leftGap  = cardRect.left  - vpLeft.value  - EDGE_MARGIN_PX
  const rightGap = vpRight.value  - cardRect.right - EDGE_MARGIN_PX

  let v = 0
  if (leftGap < 0) {
    v = leftGap * MAGNET_GAIN
  } else if (rightGap < 0) {
    v = -rightGap * MAGNET_GAIN
  }

  velocity = Math.max(-MAX_PULL_PX_PER_SEC, Math.min(MAX_PULL_PX_PER_SEC, v))
}

function onPointerLeave() {
  velocity = BASE_SPEED_PX_PER_SEC
}

const trackStyle = computed(() => ({
  transform: `translate3d(${-offset.value}px, 0, 0)`
}))

useResizeObserver(trackRef, measure)
useEventListener('resize', measure)
onMounted(async () => {
  if ('fonts' in document) await document.fonts.ready
  measure()
})
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
            :body-html="card.bodyHtml"
            :family="card.family"
            :icon="card.icon"
            :number="card.number"
            :rules="card.rules"
            :aria-hidden="copy === 2 ? 'true' : undefined"
            :tabindex="copy === 2 ? -1 : undefined"
          />
        </template>
      </div>
    </div>
  </LandingSection>
</template>
