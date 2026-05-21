<script setup lang="ts">
import { computed, ref }      from 'vue'

import { useElementMeasure }  from '../../../lib/composables/use-element-measure'

const ROT_STEP      = 67
const ROW_STRIDE_PX = 200
const STAMP_PER_COL = 240

const PERMUTATIONS: readonly (readonly string[])[] = [
  ['r','o','s','e'], ['r','o','e','s'], ['r','s','o','e'], ['r','s','e','o'],
  ['r','e','o','s'], ['r','e','s','o'], ['o','r','s','e'], ['o','r','e','s'],
  ['o','s','r','e'], ['o','s','e','r'], ['o','e','r','s'], ['o','e','s','r'],
  ['s','r','o','e'], ['s','r','e','o'], ['s','o','r','e'], ['s','o','e','r'],
  ['s','e','r','o'], ['s','e','o','r'], ['e','r','o','s'], ['e','r','s','o'],
  ['e','o','r','s'], ['e','o','s','r'], ['e','s','r','o'], ['e','s','o','r']
]

const CORNER_SIGNS: readonly [number, number][] = [[1, -1], [1, 1], [-1, 1], [-1, -1]]

const cols     = ref(6)
const layerRef = ref<HTMLElement | null>(null)
const rows     = ref(4)

function measure(): void {
  const el = layerRef.value
  if (!el) return
  cols.value = Math.max(3, Math.round(el.clientWidth / STAMP_PER_COL))
  const terminus = document.querySelector('.surfaces-carousel') as HTMLElement | null
  const parent   = el.offsetParent as HTMLElement | null
  if (terminus && parent) {
    const terminusY = terminus.getBoundingClientRect().top + window.scrollY
    const parentY   = parent.getBoundingClientRect().top   + window.scrollY
    const h         = terminusY - parentY - 40
    if (h > 0) {
      el.style.minHeight = '0'
      el.style.height    = `${h}px`
      rows.value         = Math.max(3, Math.ceil(h / ROW_STRIDE_PX) + 1)
    }
  }
}

useElementMeasure(measure, () => document.body)

interface BigStamp {
  kind   : 'big'
  rotate : number
  x      : number
  y      : number
}

interface SmallStamp {
  kind   : 'small'
  letter : string
  rotate : number
  x      : number
  y      : number
}

type Stamp = BigStamp | SmallStamp

function rotate(idx: number): number {
  return ((idx * ROT_STEP) % 360) - 180
}

const stamps = computed<readonly Stamp[]>(() => {
  const out: Stamp[] = []
  const c = cols.value
  const n = rows.value
  let idx = 0
  for (let r = 0; r < n; r++) {
    for (let cIdx = 0; cIdx < c; cIdx++) {
      const xC = ((cIdx + 0.5) / c) * 100
      const yC = (r + 0.5) * ROW_STRIDE_PX
      out.push({ kind: 'big', rotate: rotate(idx), x: xC, y: yC })
      idx++
      const o        = 0.36
      const dx       = (100 / c) * o
      const dy       = ROW_STRIDE_PX * o
      const cellSeed = ((r * 2654435761) ^ (cIdx * 40503)) >>> 0
      const shuffled = PERMUTATIONS[cellSeed % PERMUTATIONS.length]
      for (let i = 0; i < 4; i++) {
        const [sx, sy] = CORNER_SIGNS[i]
        out.push({ kind: 'small', letter: shuffled[i], rotate: rotate(idx), x: xC + sx * dx, y: yC + sy * dy })
        idx++
      }
    }
  }
  return out
})
</script>

<template>
  <div class="landing-hero">
    <div ref="layerRef" class="landing-hero-watermarks" aria-hidden="true">
      <template v-for="(s, i) in stamps" :key="i">
        <img
          v-if="s.kind === 'big'"
          src="/logo.svg"
          alt=""
          class="landing-hero-watermark landing-hero-watermark-big"
          :style="{
            '--rot' : `${s.rotate}deg`,
            left    : `${s.x}%`,
            top     : `${s.y}px`
          }"
        />
        <span
          v-else
          class="landing-hero-watermark landing-hero-watermark-small"
          :style="{
            '--rot' : `${s.rotate}deg`,
            left    : `${s.x}%`,
            top     : `${s.y}px`
          }"
        >{{ s.letter }}</span>
      </template>
    </div>
    <h1 class="landing-hero-wordmark">
      <img src="/title-with-tagline.svg" alt="Prose — A Python typesetter for the reader." />
    </h1>
  </div>
</template>
