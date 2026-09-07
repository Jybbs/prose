<script setup lang="ts">
import { useElementSize, useResizeObserver }       from '@vueuse/core'
import { computed, reactive, ref, useTemplateRef } from 'vue'
import type { CSSProperties }                      from 'vue'

import type { LengthKnob } from '../../../lib/sandbox/config-schema.data'
import * as rulerScale     from '../../../lib/sandbox/ruler-scale'
import { PALETTE }         from '../../../lib/shared/palette'

const props = defineProps<{
  lengths : readonly LengthKnob[]
  valueOf : (key: string) => number
}>()

const emit = defineEmits<{
  dragging  : [key: string, hue: string],
  preview   : [key: string, value: number],
  setLength : [key: string, value: number]
}>()

const MAX  = rulerScale.LENGTH_MAX
const MIN  = rulerScale.LENGTH_MIN
const SPAN = MAX - MIN

const DRAG_SLOP = 3
const HUE_NAMES: readonly (keyof typeof PALETTE)[] = ['ube', 'dexter', 'whiskey']
const MARKS     = [MIN, 80, 130, MAX] as const

interface DragState {
  key       : string
  live      : boolean
  originX   : number
  perChar   : number
  pointerId : number
  startX    : number
}

const chipEls    = useTemplateRef<HTMLElement[]>('chip')
const chipWidths = reactive(new Map<string, number>())
const drag       = ref<DragState | null>(null)
const draft      = ref('')
const editing    = ref('')
const preview    = ref<{ key: string, value: number } | null>(null)
const trackEl    = ref<HTMLElement | null>(null)

// A drag moves this preview alone, the config committing on release, so the
// code and toml surfaces hold still while a stop is in hand.
function shownValue(knob: LengthKnob): number {
  const held = preview.value
  return held && held.key === knob.key ? held.value : props.valueOf(knob.key)
}

const { width: trackWidth } = useElementSize(trackEl)

useResizeObserver(chipEls, entries => {
  for (const { target } of entries) {
    const chip = target as HTMLElement
    if (chip.dataset.key !== undefined) chipWidths.set(chip.dataset.key, chip.offsetWidth)
  }
})

const tierByKey = computed(() => rulerScale.packTiers(
  props.lengths.map(knob => ({ key: knob.key, pct: rulerScale.pctOfLength(shownValue(knob)) })),
  trackWidth.value,
  chipWidths
))

// The head reserves rows only for tiers in use, so it stays one row tall
// until chips actually crowd.
const rows = computed(() => {
  let top = 0
  for (const tier of tierByKey.value.values()) top = Math.max(top, tier)
  return top + 1
})

function beginEdit(knob: LengthKnob): void {
  drag.value    = null
  draft.value   = String(props.valueOf(knob.key))
  editing.value = knob.key
}

function cancelEdit(): void {
  editing.value = ''
}

function commitEdit(knob: LengthKnob): void {
  if (editing.value !== knob.key) return
  const parsed = Number.parseInt(draft.value, 10)
  if (Number.isFinite(parsed)) emit('setLength', knob.key, rulerScale.clampLength(parsed))
  editing.value = ''
}

function dragEnd(event: PointerEvent): void {
  if (drag.value?.pointerId !== event.pointerId) return
  const held = preview.value
  if (held) emit('setLength', held.key, held.value)
  preview.value = null
  drag.value    = null
  emit('dragging', '', '')
}

function dragMove(knob: LengthKnob, event: PointerEvent): void {
  const state = drag.value
  if (!state || state.key !== knob.key || state.pointerId !== event.pointerId) return
  if (!state.live && Math.abs(event.clientX - state.startX) < DRAG_SLOP) return
  state.live = true
  const value = rulerScale.clampLength((event.clientX - state.originX) / state.perChar + MIN)
  preview.value = { key: knob.key, value }
  emit('preview', knob.key, value)
}

function dragStart(hue: string, knob: LengthKnob, event: PointerEvent): void {
  if (editing.value === knob.key || event.button !== 0 || !trackEl.value) return
  event.preventDefault()
  const stop = event.currentTarget as HTMLElement
  const rect = trackEl.value.getBoundingClientRect()
  const unit = rect.width / SPAN
  stop.setPointerCapture(event.pointerId)
  emit('dragging', knob.key, hue)
  drag.value = {
    key       : knob.key,
    live      : false,
    originX   : event.clientX - (rulerScale.clampLength(props.valueOf(knob.key)) - MIN) * unit,
    perChar   : unit,
    pointerId : event.pointerId,
    startX    : event.clientX
  }
}

function focusInput(el: unknown): void {
  if (el instanceof HTMLInputElement && el !== document.activeElement) el.select()
}

function hueOf(index: number): string {
  return `var(--prose-palette-${HUE_NAMES[index % HUE_NAMES.length]})`
}

function keyStep(knob: LengthKnob, event: KeyboardEvent): void {
  const next = rulerScale.steppedLength(event.key, event.shiftKey, props.valueOf(knob.key))
  if (next === null) return
  event.preventDefault()
  emit('setLength', knob.key, rulerScale.clampLength(next))
}

function stopStyle(index: number, knob: LengthKnob): CSSProperties {
  return {
    '--hue'  : hueOf(index),
    '--pct'  : rulerScale.pctOfLength(shownValue(knob)),
    '--tier' : tierOf(knob.key)
  }
}


function tierOf(key: string): number {
  return tierByKey.value.get(key) ?? 0
}
</script>

<template>
  <div class="sandbox-ruler-head">
    <div ref="trackEl" class="sandbox-ruler" role="group" aria-label="Line lengths" :style="{ '--rows': rows }">
      <div class="sandbox-ruler-track" aria-hidden="true" />
      <span
        v-for="(knob, index) in lengths"
        :key="`default-${knob.key}`"
        class="sandbox-ruler-default"
        aria-hidden="true"
        :style="{ left: `${rulerScale.pctOfLength(knob.default)}%`, '--hue': hueOf(index) }"
      />
      <span
        v-for="mark in MARKS"
        :key="mark"
        class="sandbox-ruler-num"
        aria-hidden="true"
        :style="{ left: `${rulerScale.pctOfLength(mark)}%` }"
      >{{ mark }}</span>
      <div
        v-for="(knob, index) in lengths"
        :key="knob.key"
        class="sandbox-ruler-stop"
        :style="stopStyle(index, knob)"
        :data-dragging="drag?.key === knob.key || null"
        :role="editing === knob.key ? undefined : 'slider'"
        :tabindex="editing === knob.key ? undefined : 0"
        :aria-label="`${knob.label} line length`"
        :aria-valuemin="MIN"
        :aria-valuemax="MAX"
        :aria-valuenow="shownValue(knob)"
        @pointerdown="dragStart(hueOf(index), knob, $event)"
        @pointermove="dragMove(knob, $event)"
        @pointerup="dragEnd"
        @pointercancel="dragEnd"
        @keydown="keyStep(knob, $event)"
      >
        <span class="sandbox-ruler-marker" aria-hidden="true" />
        <span
          v-if="editing !== knob.key"
          ref="chip"
          class="sandbox-ruler-chip"
          :data-key="knob.key"
          @dblclick="beginEdit(knob)"
        >
          <span class="sandbox-ruler-chip-label">{{ knob.label }}</span>
          <span class="sandbox-ruler-chip-value">{{ shownValue(knob) }}</span>
        </span>
        <input
          v-else
          :ref="focusInput"
          v-model="draft"
          type="number"
          class="panel-number sandbox-ruler-chip sandbox-ruler-chip-input"
          :min="MIN"
          :max="MAX"
          :aria-label="`${knob.label} line length`"
          @keydown.stop
          @keydown.enter.prevent="commitEdit(knob)"
          @keydown.esc="cancelEdit"
          @blur="commitEdit(knob)"
        >
      </div>
    </div>
  </div>
</template>
