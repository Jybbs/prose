<script setup lang="ts">
import { useElementBounding }       from '@vueuse/core'
import { computed, useTemplateRef } from 'vue'

import { tileStamps }                from '../../../lib/landing/hero-stamps'
import { heroGrid, watermarkHeight } from '../../../lib/landing/hero-tiling'
import { SITE_ALT }                  from '../../../lib/shared/constants'

const heroRef  = useTemplateRef<HTMLElement>('hero')
const layerRef = useTemplateRef<HTMLElement>('layer')

const { top:   heroTop }     = useElementBounding(heroRef)
const { width: layerWidth }  = useElementBounding(layerRef)
const { top:   terminusTop } = useElementBounding(() =>
  typeof document === 'undefined' ? null : document.querySelector<HTMLElement>('.surfaces-carousel')
)

const layerHeight = computed(() => {
  const height = watermarkHeight(heroTop.value, terminusTop.value)
  return height > 0 ? height : null
})

const layerStyle = computed(() =>
  layerHeight.value !== null ? { height: `${layerHeight.value}px`, minHeight: '0' } : undefined
)

const grid   = computed(() => heroGrid(layerHeight.value ?? 0, layerWidth.value))
const stamps = computed(() => tileStamps(grid.value.cols, grid.value.rows))
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
            '--rotation' : `${s.rotate}deg`,
            left         : `${s.x}%`,
            top          : `${s.y}px`
          }"
        />
        <span
          v-else
          class="landing-hero-watermark landing-hero-watermark-small"
          :style="{
            '--rotation' : `${s.rotate}deg`,
            left         : `${s.x}%`,
            top          : `${s.y}px`
          }"
        >{{ s.letter }}</span>
      </template>
    </div>
    <h1 class="landing-hero-wordmark">
      <img src="/title-with-tagline.svg" :alt="SITE_ALT" />
    </h1>
  </div>
</template>
