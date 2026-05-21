<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'

import LandingSection from '../LandingSection.vue'

import { data as landing } from '../../../../data/landing.data'
import { data as rules }   from '../../../../data/rules.data'

import SurfaceCardTabIndex from './SurfaceCardTabIndex.vue'

const surfaceCards = computed(() => {
  const allGroups = rules.byCategory.flatMap(c => c.byDomain)
  return landing.surfaces.map(s => {
    const group = allGroups.find(g => g.domain === s.domain)
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

const carouselRef = ref<HTMLElement | null>(null)
const trackRef    = ref<HTMLElement | null>(null)
const offset      = ref(0)
const isDragging  = ref(false)

const AUTO_SPEED_PX_PER_SEC = 32
const CLICK_DRAG_THRESHOLD  = 5
const COAST_TAU_SEC         = 0.32
const COAST_EXIT_PX_PER_SEC = AUTO_SPEED_PX_PER_SEC * 0.8
const COAST_MAX_PX_PER_SEC  = 2400
const VELOCITY_WINDOW_MS    = 80

type VelocitySample = { time: number; x: number }

let halfWidth        = 0
let rafId            = 0
let lastFrameTime    = 0
let hovered          = false
let pointerId        = -1
let lastDragX        = 0
let dragTotalDelta   = 0
let suppressClick    = false
let reducedMotion    = false
let coasting         = false
let coastVelocity    = 0
let velocitySamples: VelocitySample[] = []

function measure() {
  const track = trackRef.value
  if (!track) return
  const originalCount = surfaceCards.value.length
  const cards         = track.children
  if (cards.length >= originalCount + 1) {
    const firstCard     = cards[0] as HTMLElement
    const firstDupCard  = cards[originalCount] as HTMLElement
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

function pruneSamples(now: number) {
  const cutoff = now - VELOCITY_WINDOW_MS
  while (velocitySamples.length > 0 && velocitySamples[0].time < cutoff) {
    velocitySamples.shift()
  }
}

function computeReleaseVelocity(now: number): number {
  pruneSamples(now)
  if (velocitySamples.length < 2) return 0
  const oldest = velocitySamples[0]
  const latest = velocitySamples[velocitySamples.length - 1]
  const dtMs   = latest.time - oldest.time
  if (dtMs <= 0) return 0
  const vx = ((latest.x - oldest.x) / dtMs) * 1000
  if (vx >  COAST_MAX_PX_PER_SEC) return  COAST_MAX_PX_PER_SEC
  if (vx < -COAST_MAX_PX_PER_SEC) return -COAST_MAX_PX_PER_SEC
  return vx
}

function tick(now: number) {
  if (lastFrameTime === 0) lastFrameTime = now
  const dt = (now - lastFrameTime) / 1000
  lastFrameTime = now

  if (halfWidth > 0 && !isDragging.value) {
    if (coasting) {
      if (hovered || reducedMotion) {
        coasting      = false
        coastVelocity = 0
      } else {
        offset.value = wrap(offset.value - coastVelocity * dt)
        coastVelocity *= Math.exp(-dt / COAST_TAU_SEC)
        if (Math.abs(coastVelocity) < COAST_EXIT_PX_PER_SEC) {
          coasting      = false
          coastVelocity = 0
        }
      }
    } else if (!hovered && !reducedMotion) {
      offset.value = wrap(offset.value + AUTO_SPEED_PX_PER_SEC * dt)
    }
  }

  rafId = requestAnimationFrame(tick)
}

function onPointerEnter() {
  hovered = true
}

function onPointerLeave() {
  hovered = false
}

function onPointerDown(event: PointerEvent) {
  if (event.button !== 0 && event.pointerType === 'mouse') return
  const target = carouselRef.value
  if (!target) return

  measure()
  coasting         = false
  coastVelocity    = 0
  isDragging.value = true
  pointerId        = event.pointerId
  lastDragX        = event.clientX
  dragTotalDelta   = 0
  suppressClick    = false
  velocitySamples  = [{ time: event.timeStamp, x: event.clientX }]
  target.setPointerCapture(event.pointerId)
}

function onPointerMove(event: PointerEvent) {
  if (!isDragging.value || event.pointerId !== pointerId) return
  const dx = event.clientX - lastDragX
  lastDragX = event.clientX
  dragTotalDelta += Math.abs(dx)
  if (dragTotalDelta > CLICK_DRAG_THRESHOLD) {
    suppressClick = true
    event.preventDefault()
  }

  offset.value = wrap(offset.value - dx)

  velocitySamples.push({ time: event.timeStamp, x: event.clientX })
  pruneSamples(event.timeStamp)
}

function endDrag(event: PointerEvent) {
  if (event.pointerId !== pointerId) return
  const target = carouselRef.value
  if (target && target.hasPointerCapture(event.pointerId)) {
    target.releasePointerCapture(event.pointerId)
  }

  const releaseVelocity = computeReleaseVelocity(event.timeStamp)
  if (Math.abs(releaseVelocity) >= COAST_EXIT_PX_PER_SEC) {
    coastVelocity = releaseVelocity
    coasting      = true
  } else {
    coastVelocity = 0
    coasting      = false
  }
  velocitySamples = []

  isDragging.value = false
  pointerId        = -1
}

function onClickCapture(event: MouseEvent) {
  if (suppressClick) {
    event.preventDefault()
    event.stopPropagation()
    suppressClick = false
  }
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
      ref="carouselRef"
      class="surfaces-carousel"
      :class="{ 'is-dragging': isDragging }"
      aria-label="Rule family carousel"
      @pointerenter="onPointerEnter"
      @pointerleave="onPointerLeave"
      @pointerdown="onPointerDown"
      @pointermove="onPointerMove"
      @pointerup="endDrag"
      @pointercancel="endDrag"
      @click.capture="onClickCapture"
    >
      <div
        ref="trackRef"
        class="surfaces-carousel-track"
        :style="trackStyle"
      >
        <SurfaceCardTabIndex
          v-for="(card, idx) in doubled"
          :key="`${idx}-${card.domain}`"
          :body-html="card.bodyHtml"
          :domain="card.domain"
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
