<script setup lang="ts">
import { useElementSize, useMouseInElement, useRafFn, useScroll, useTimeoutFn } from '@vueuse/core'
import { computed, ref, useTemplateRef, watch }                                 from 'vue'

import { useHiddenTabindex } from '../../../../lib/composables/use-aria-hidden'
import { useReducedMotion }  from '../../../../lib/composables/use-reduced-motion'
import type { RenderedRule } from '../../../../lib/rules/rules.data'
import { MS_PER_SEC }        from '../../../../lib/shared/constants'
import SurfacePip            from './SurfacePip.vue'
import SurfaceRailName       from './SurfaceRailName.vue'

const EDGE_PX      = 34
const GLIDE_PX_SEC = 260
const JUMP_MS      = 420
const PINNED_MIN   = 3
const SETTLE_SEC   = 0.09
const SLACK_PX     = 1
const SLOT_PX      = 24

const props = defineProps<{ rules: readonly RenderedRule[] }>()

const railRef       = useTemplateRef<HTMLElement>('rail')
const windowRef     = useTemplateRef<HTMLElement>('window')
const reducedMotion = useReducedMotion()

const { width: railWidth }    = useElementSize(railRef)
const { width: windowWidth }  = useElementSize(windowRef)
const { elementX, isOutside } = useMouseInElement(windowRef)
const { x: scrolled }         = useScroll(windowRef)

const heading    = ref(1)
const hoveredIdx = ref<number | null>(null)
const jumping    = ref(false)

const activeIdx  = computed(() => hoveredIdx.value ?? 0)
const activeRule = computed(() => props.rules[activeIdx.value])
const hovered    = computed(() => hoveredIdx.value !== null)
const tabindex   = useHiddenTabindex()

const overflows = computed(() => railWidth.value > 0 && SLOT_PX * props.rules.length > railWidth.value)
const pinned    = computed(() => overflows.value && props.rules.length > PINNED_MIN)

const inner = computed(() => {
  const indices = props.rules.map((_, index) => index)
  return pinned.value ? indices.slice(1, -1) : indices
})

const travel  = computed(() => Math.max(0, SLOT_PX * inner.value.length - windowWidth.value))
const travels = computed(() => travel.value > SLACK_PX)

const atStart = computed(() => scrolled.value <= 0.5)
const atEnd   = computed(() => scrolled.value >= travel.value - 0.5)
const behind  = computed(() => (travels.value ? Math.min(1, Math.max(0, scrolled.value / travel.value)) : 0))

const chevrons = computed(() => (travels.value ? [
  { at: atStart.value, glyph: '‹', label: 'Travel to the first rule', reach: behind.value,     side: 'start', toEnd: false },
  { at: atEnd.value,   glyph: '›', label: 'Travel to the last rule',  reach: 1 - behind.value, side: 'end',   toEnd: true  }
] : []))

const edge = computed(() => {
  if (isOutside.value || jumping.value || !travels.value) return 'none'
  if (elementX.value < EDGE_PX && !atStart.value) return 'start'
  if (windowWidth.value - elementX.value < EDGE_PX && !atEnd.value) return 'end'
  return 'none'
})

const depth = computed(() => {
  if (edge.value === 'start') return 1 - Math.max(0, elementX.value) / EDGE_PX
  if (edge.value === 'end')   return 1 - Math.max(0, windowWidth.value - elementX.value) / EDGE_PX
  return 0
})

const velocity = computed(() => {
  if (edge.value === 'none') return 0
  return (edge.value === 'start' ? -GLIDE_PX_SEC : GLIDE_PX_SEC) * depth.value ** 2
})

let applied = 0

function pipBinding(index: number) {
  return {
    active   : hovered.value && index === activeIdx.value,
    distance : hovered.value ? Math.abs(index - activeIdx.value) : 0,
    index,
    rule     : props.rules[index]
  }
}

function select(index: number): void {
  heading.value    = index < activeIdx.value ? -1 : 1
  hoveredIdx.value = index
}

function selectUnderPointer(): void {
  if (isOutside.value) return
  const index = inner.value[Math.min(
    inner.value.length - 1,
    Math.max(0, Math.floor(((windowRef.value?.scrollLeft ?? 0) + elementX.value) / SLOT_PX))
  )]
  if (index !== undefined && index !== activeIdx.value) select(index)
}

const { pause, resume } = useRafFn(({ delta }) => {
  const el = windowRef.value
  if (!el) return
  const step = delta / MS_PER_SEC
  applied = reducedMotion.value
    ? velocity.value
    : applied + (velocity.value - applied) * (1 - Math.exp(-step / SETTLE_SEC))
  if (velocity.value === 0 && Math.abs(applied) < 1) {
    applied = 0
    pause()
    return
  }
  el.scrollLeft += applied * step
  selectUnderPointer()
}, { immediate: false })

const { start: releaseJump } = useTimeoutFn(() => { jumping.value = false }, JUMP_MS, { immediate: false })

watch(velocity, moving => { if (moving !== 0) resume() }, { immediate: true })

function jump(toEnd: boolean): void {
  const el = windowRef.value
  if (!el) return
  jumping.value = true
  applied       = 0
  el.scrollTo({
    behavior : reducedMotion.value ? 'auto' : 'smooth',
    left     : toEnd ? el.scrollWidth : 0
  })
  releaseJump()
}

const swap = computed(() => (heading.value < 0 ? 'surface-rail-back' : 'surface-rail-fwd'))

const railStyle = computed(() => ({
  '--rail-inset' : pinned.value ? `${SLOT_PX}px` : '0px',
  '--slot'       : `${SLOT_PX}px`
}))

const windowStyle = computed(() => ({
  '--fade-end'   : atEnd.value   ? '0px' : '20px',
  '--fade-start' : atStart.value ? '0px' : '20px'
}))
</script>

<template>
  <div class="surface-rail">
    <div ref="rail" class="surface-rail-row" :data-bookends="pinned" :style="railStyle">
      <SurfacePip
        v-if="pinned"
        class="surface-rail-end"
        v-bind="pipBinding(0)"
        @select="select(0)"
      />
      <div ref="window" class="surface-rail-window" :data-edge="edge" :style="windowStyle">
        <div class="surface-rail-track">
          <SurfacePip
            v-for="index in inner"
            :key="rules[index].slug"
            v-bind="pipBinding(index)"
            @select="select(index)"
          />
        </div>
      </div>
      <button
        v-for="chevron in chevrons"
        :key="chevron.side"
        type="button"
        class="surface-rail-chevron"
        :data-side="chevron.side"
        :disabled="chevron.at"
        :style="{ '--reach': chevron.reach }"
        :tabindex="tabindex"
        :aria-label="chevron.label"
        @click="jump(chevron.toEnd)"
      >{{ chevron.glyph }}</button>
      <SurfacePip
        v-if="pinned"
        class="surface-rail-end"
        v-bind="pipBinding(rules.length - 1)"
        @select="select(rules.length - 1)"
      />
    </div>
    <div class="surface-rail-name" aria-live="polite">
      <SurfaceRailName :rule="activeRule" :swap="swap" />
    </div>
  </div>
</template>
