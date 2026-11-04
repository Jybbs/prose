<script setup lang="ts">
import { useElementSize, useResizeObserver }       from '@vueuse/core'
import { computed, reactive, ref, useTemplateRef } from 'vue'
import type { CSSProperties }                      from 'vue'

import type { LengthKnob } from '../../../lib/sandbox/config-schema.data'
import * as rulerScale     from '../../../lib/sandbox/ruler-scale'

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
const HUES      = ['ube', 'dexter', 'whiskey'] as const
const LANE_SLOP = 6
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

// Joins each knob's default to its value, a tether that would overlap another
// dropping to the next lane.
const tethers = computed(() => rulerScale.packLanes(
  new Map(props.lengths.map(knob => {
    const [at, home] = [shownValue(knob), knob.default].map(rulerScale.pctOfLength) as [number, number]
    return [knob.key, { from: Math.min(at, home), to: Math.max(at, home) }]
  })),
  trackWidth.value,
  LANE_SLOP
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

// Gives each knob its hue by its place, so a knob, its default, its tether,
// and the guide it holds share one.
function hueOf(index: number): string {
  return HUES[index % HUES.length]!
}

function focusInput(el: unknown): void {
  if (el instanceof HTMLInputElement && el !== document.activeElement) el.select()
}

function keyStep(knob: LengthKnob, event: KeyboardEvent): void {
  const next = rulerScale.steppedLength(event.key, event.shiftKey, props.valueOf(knob.key))
  if (next === null) return
  event.preventDefault()
  emit('setLength', knob.key, rulerScale.clampLength(next))
}

function stopStyle(knob: LengthKnob): CSSProperties {
  return {
    '--pct'  : rulerScale.pctOfLength(shownValue(knob)),
    '--tier' : tierOf(knob.key)
  }
}

function tetherStyle(knob: LengthKnob): CSSProperties & { '--lane': number } {
  const { from, lane, to } = tethers.value.get(knob.key)!
  return { '--lane': lane, 'left': `${from}%`, 'width': `${to - from}%` }
}

function tierOf(key: string): number {
  return tierByKey.value.get(key) ?? 0
}
</script>

<template>
  <div class="flex flex-wrap items-center gap-x-1-25rem gap-y-0-75rem">
    <div ref="trackEl" class="sandbox-ruler" role="group" aria-label="Line lengths" :style="{ '--rows': rows }">
      <div class="sandbox-ruler-track" aria-hidden="true" />
      <span
        v-for="(knob, index) in lengths"
        :key="`tether-${knob.key}`"
        class="sandbox-ruler-tether pointer-events-none absolute h-2"
        aria-hidden="true"
        :data-hue="hueOf(index)"
        :style="tetherStyle(knob)"
      />
      <span
        v-for="(knob, index) in lengths"
        :key="`default-${knob.key}`"
        class="sandbox-ruler-default"
        aria-hidden="true"
        :data-hue="hueOf(index)"
        :style="{ '--lane': tetherStyle(knob)['--lane'], 'left': `${rulerScale.pctOfLength(knob.default)}%` }"
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
        class="sandbox-ruler-stop group focus-within:z-2 hover:z-2 data-dragging:z-2"
        :style="stopStyle(knob)"
        :data-changed="shownValue(knob) !== knob.default || null"
        :data-dragging="drag?.key === knob.key || null"
        :data-hue="hueOf(index)"
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
        <span class="sandbox-ruler-marker before:absolute before:inset-x-4 before:inset-y-0 before:transition-colors before:bg-text-2 group-hover:before:bg-text-1 group-focus-visible:before:bg-text-1 group-data-dragging:before:bg-text-1" aria-hidden="true" />
        <span
          v-if="editing !== knob.key"
          ref="chip"
          class="sandbox-ruler-chip group-focus-visible:focus-ring group-focus-visible:outline-offset-1 group-hover:border-text-2 group-data-dragging:border-text-2"
          :data-key="knob.key"
          @dblclick="beginEdit(knob)"
        >
          <span class="font-mono text-meta tracking-0-05em text-text-2 uppercase">{{ knob.label }}</span>
          <span class="font-mono text-0-68rem text-text-1 tabular-nums transition-colors group-data-changed:text-knob-figure">{{ shownValue(knob) }}</span>
        </span>
        <input
          v-else
          :ref="focusInput"
          v-model="draft"
          type="number"
          class="sandbox-ruler-chip sandbox-ruler-chip-input panel-number w-3-4rem cursor-text px-5 font-mono text-0-68rem text-text-1 select-text focus-visible:-outline-offset-1 group-hover:border-text-2 group-data-dragging:border-text-2"
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
