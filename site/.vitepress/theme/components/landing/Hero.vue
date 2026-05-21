<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'

const STAMP_PER_COL  = 240
const ROW_STRIDE_PX  = 200
const ROT_STEP       = 67
const LETTERS        = ['r', 'o', 's', 'e'] as const

const layerRef = ref<HTMLElement | null>(null)
const cols     = ref(6)
const rows     = ref(4)

let bodyObserver: ResizeObserver | null = null

function measure() {
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

onMounted(() => {
  if (typeof window === 'undefined' || !layerRef.value) return
  bodyObserver = new ResizeObserver(measure)
  bodyObserver.observe(document.body)
  requestAnimationFrame(measure)
  if ('fonts' in document) document.fonts.ready.then(() => requestAnimationFrame(measure))
})

onBeforeUnmount(() => bodyObserver?.disconnect())

interface BigStamp   { kind : 'big'  ; rotate : number; x : number; y : number }
interface SmallStamp { kind : 'small'; letter : string; rotate : number; x : number; y : number }
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
      const corners: [number, number][] = [
        [xC + dx, yC - dy],
        [xC + dx, yC + dy],
        [xC - dx, yC + dy],
        [xC - dx, yC - dy]
      ]
      const cellSeed = ((r * 2654435761) ^ (cIdx * 40503)) >>> 0
      const shuffled = [...LETTERS]
      for (let k = shuffled.length - 1; k > 0; k--) {
        const j = (((cellSeed >>> (k * 3)) ^ ((cellSeed * (k + 1)) >>> 0)) >>> 0) % (k + 1)
        ;[shuffled[k], shuffled[j]] = [shuffled[j], shuffled[k]]
      }
      for (let i = 0; i < 4; i++) {
        const [x, y] = corners[i]
        out.push({ kind: 'small', letter: shuffled[i], rotate: rotate(idx), x, y })
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
            left    : `${s.x}%`,
            top     : `${s.y}px`,
            '--rot' : `${s.rotate}deg`
          }"
        />
        <span
          v-else
          class="landing-hero-watermark landing-hero-watermark-small"
          :style="{
            left    : `${s.x}%`,
            top     : `${s.y}px`,
            '--rot' : `${s.rotate}deg`
          }"
        >{{ s.letter }}</span>
      </template>
    </div>
    <h1 class="landing-hero-wordmark">
      <img src="/title-with-tagline.svg" alt="Prose — A Python typesetter for the reader." />
    </h1>
  </div>
</template>
