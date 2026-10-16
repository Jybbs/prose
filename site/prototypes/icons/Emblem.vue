<script setup lang="ts">
import { computed, useId } from 'vue'

import type { Emblem } from '../../.vitepress/lib/shared/emblems'
import * as emblems    from './emblems'

const TIP_FILLS = { cut: '#000', ink: 'currentColor' } as const

const props = defineProps<{
  emblem      : Emblem
  house       : emblems.House
  standalone ?: boolean
}>()

const id      = useId()
const nib     = computed(() => props.house.terminal === 'nib')
const line    = computed(() => props.house.form === 'line')
const painted = computed(() => !line.value || props.house.tone === 'duotone')

const tips = (path: string, marker: keyof typeof TIP_FILLS): Record<string, string> => {
  if (!nib.value || emblems.isClosed(path)) return {}
  const url = `url(#${id}-${marker})`
  return { 'marker-end': url, 'marker-start': url }
}

const strokes = computed(() => {
  const { cuts, lines, shapes } = props.emblem.motif
  return line.value ? [...shapes, ...cuts, ...lines] : lines
})
</script>

<template>
  <svg
    class="glyph icon-emblem"
    viewBox="0 0 24 24"
    fill="none"
    :data-family="emblem.family"
    :data-form="house.form"
    :data-terminal="house.terminal"
    :data-tone="house.tone"
    :style="{ strokeWidth: house.weight }"
    :role="standalone ? 'img' : undefined"
    :aria-label="standalone ? emblem.motif.label : undefined"
    :aria-hidden="standalone ? undefined : 'true'"
  >
    <defs>
      <template v-if="nib">
        <marker
          v-for="(fill, tip) in TIP_FILLS"
          :id="`${id}-${tip}`"
          :key="tip"
          markerUnits="strokeWidth"
          markerWidth="0.55"
          markerHeight="1"
          orient="auto-start-reverse"
          viewBox="-0.05 -0.5 0.55 1"
        >
          <path d="M-0.05-0.5 0.5 0-0.05 0.5Z" stroke="none" :fill="fill" />
        </marker>
      </template>
      <mask v-if="painted" :id="`${id}-mask`">
        <rect width="24" height="24" fill="#fff" stroke="none" />
        <path
          v-for="cut in emblem.motif.cuts"
          :key="cut"
          stroke="#000"
          :d="cut"
          :fill="emblems.isClosed(cut) ? '#000' : 'none'"
          v-bind="tips(cut, 'cut')"
        />
      </mask>
    </defs>
    <g v-if="painted" class="icon-emblem-paint" :mask="`url(#${id}-mask)`">
      <path v-for="shape in emblem.motif.shapes" :key="shape" :d="shape" />
    </g>
    <g v-if="!line" class="icon-emblem-outline">
      <path v-for="shape in emblem.motif.shapes" :key="shape" :d="shape" />
    </g>
    <path
      v-for="stroke in strokes"
      :key="stroke"
      :d="stroke"
      v-bind="tips(stroke, 'ink')"
    />
  </svg>
</template>
