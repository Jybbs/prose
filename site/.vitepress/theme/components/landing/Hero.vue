<script setup lang="ts">
import { useElementBounding }       from '@vueuse/core'
import { computed, useTemplateRef } from 'vue'

import { ROW_STRIDE_PX, tileStamps } from '../../../lib/landing/hero-stamps'

const STAMP_PER_COL = 240
const TERMINUS_GAP  = 40

const heroRef  = useTemplateRef<HTMLElement>('hero')
const layerRef = useTemplateRef<HTMLElement>('layer')

const { top:   heroTop }     = useElementBounding(heroRef)
const { width: layerWidth }  = useElementBounding(layerRef)
const { top:   terminusTop } = useElementBounding(() =>
  typeof document === 'undefined' ? null : document.querySelector<HTMLElement>('.surfaces-carousel')
)

const layerHeight = computed(() => {
  const h = terminusTop.value - heroTop.value - TERMINUS_GAP
  return h > 0 ? h : null
})

const layerStyle = computed(() =>
  layerHeight.value !== null ? { height: `${layerHeight.value}px`, minHeight: '0' } : undefined
)

const cols = computed(() => Math.max(3, Math.round(layerWidth.value / STAMP_PER_COL)))
const rows = computed(() => Math.max(3, Math.ceil((layerHeight.value ?? 0) / ROW_STRIDE_PX) + 1))

const stamps = computed(() => tileStamps(cols.value, rows.value))
</script>

<template>
  <div ref="hero" class="landing-hero">
    <div ref="layer" class="landing-hero-watermarks" :style="layerStyle" aria-hidden="true">
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
      <img src="/title-with-tagline.svg" alt="Prose, a Python typesetter for the reader." />
    </h1>
  </div>
</template>
