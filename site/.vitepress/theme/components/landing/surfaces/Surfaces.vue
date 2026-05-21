<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'

import LandingSection from '../LandingSection.vue'

import { data as landing } from '../../../../data/landing.data'
import { data as rules }   from '../../../../data/rules.data'

import SurfaceCardTabIndex from './SurfaceCardTabIndex.vue'

const surfaceCards = computed(() => {
  const allGroups = rules.byCategory.flatMap(c => c.byFamily)
  return landing.surfaces.map(s => {
    const group = allGroups.find(g => g.family === s.family)
    return { ...s, rules: group?.rules ?? [] }
  })
})

const doubled = computed(() => [
  ...surfaceCards.value.map(c => ({ ...c, dup: false })),
  ...surfaceCards.value.map(c => ({ ...c, dup: true  }))
])

const ruleCount  = computed(() => rules.list.length)
const familyWord = computed(() => surfaceCards.value.length === 1 ? 'rule family' : 'rule families')

const heading = computed(() =>
  `<strong>${surfaceCards.value.length}</strong> ${familyWord.value}. <em>${ruleCount.value}</em> rules.`
)

const trackRef = ref<HTMLElement | null>(null)
const offset   = ref(0)

const BASE_SPEED_PX_PER_SEC = 32
const MAX_PULL_PX_PER_SEC   = BASE_SPEED_PX_PER_SEC * 8
const MAGNET_GAIN           = 4
const EDGE_MARGIN_PX        = 32

let halfWidth     = 0
let rafId         = 0
let lastFrameTime = 0
let velocity      = BASE_SPEED_PX_PER_SEC
let reducedMotion = false

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

function tick(now: number) {
  if (lastFrameTime === 0) lastFrameTime = now
  const dt = (now - lastFrameTime) / 1000
  lastFrameTime = now

  if (halfWidth > 0 && !reducedMotion) {
    offset.value = wrap(offset.value + velocity * dt)
  }

  rafId = requestAnimationFrame(tick)
}

function onPointerMove(event: PointerEvent) {
  const node = (event.target as HTMLElement).closest('.surface-card') as HTMLElement | null
  if (!node || !trackRef.value) {
    velocity = 0
    return
  }
  const viewport = trackRef.value.parentElement
  if (!viewport) return

  const cardRect = node.getBoundingClientRect()
  const vpRect   = viewport.getBoundingClientRect()
  const leftGap  = cardRect.left  - vpRect.left  - EDGE_MARGIN_PX
  const rightGap = vpRect.right   - cardRect.right - EDGE_MARGIN_PX

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

let resizeObserver: ResizeObserver | null = null
let motionQuery: MediaQueryList | null = null
function syncMotion() {
  reducedMotion = motionQuery?.matches ?? false
}

onMounted(() => {
  if (typeof window === 'undefined') return
  motionQuery = window.matchMedia('(prefers-reduced-motion: reduce)')
  syncMotion()
  motionQuery.addEventListener('change', syncMotion)

  measure()
  if (trackRef.value && 'ResizeObserver' in window) {
    resizeObserver = new ResizeObserver(() => measure())
    resizeObserver.observe(trackRef.value)
  }
  window.addEventListener('resize', measure)
  if ('fonts' in document) {
    document.fonts.ready.then(measure)
  }

  rafId = requestAnimationFrame(tick)
})

onBeforeUnmount(() => {
  if (rafId) cancelAnimationFrame(rafId)
  if (resizeObserver) resizeObserver.disconnect()
  if (typeof window !== 'undefined') {
    window.removeEventListener('resize', measure)
  }
  if (motionQuery) motionQuery.removeEventListener('change', syncMotion)
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
      class="surfaces-carousel"
      aria-label="Rule family carousel"
      @pointermove="onPointerMove"
      @pointerleave="onPointerLeave"
    >
      <div
        ref="trackRef"
        class="surfaces-carousel-track"
        :style="trackStyle"
      >
        <SurfaceCardTabIndex
          v-for="(card, idx) in doubled"
          :key="`${idx}-${card.family}`"
          :body-html="card.bodyHtml"
          :family="card.family"
          :icon="card.icon"
          :number="card.number"
          :rules="card.rules"
          :aria-hidden="card.dup ? 'true' : undefined"
          :tabindex="card.dup ? -1 : undefined"
        />
      </div>
    </div>
  </LandingSection>
</template>
