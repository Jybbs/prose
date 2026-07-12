<script setup lang="ts">
import { useElementSize }                           from '@vueuse/core'
import { computed, onBeforeUnmount, reactive, ref } from 'vue'
import type { CSSProperties }                       from 'vue'

import type { LengthKnob } from '../../../lib/sandbox/config-schema.data'

const props = defineProps<{
  lengths : readonly LengthKnob[]
  valueOf : (key: string) => number
}>()

const emit = defineEmits<{
  dragging  : [key: string],
  preview   : [key: string, value: number],
  setLength : [key: string, value: number]
}>()

const CHIP_FALLBACK  = 72
const CLEARANCE      = 8
const DRAG_SLOP      = 3
const HUE_NAMES      = ['ube', 'dexter', 'whiskey'] as const
const MARKS          = [30, 80, 130, 180] as const
const MAX            = 180
const MIN            = 30
const SPAN           = MAX - MIN
const TRACK_FALLBACK = 480

interface DragState {
  key       : string
  live      : boolean
  originX   : number
  perChar   : number
  pointerId : number
  startX    : number
}

interface Stop {
  key : string
  pct : number
}

const chipKeys   = new WeakMap<Element, string>()
const chipWidths = reactive(new Map<string, number>())
const drag       = ref<DragState | null>(null)
const draft      = ref('')
const editing    = ref('')
const observed   = new WeakSet<HTMLElement>()
const preview    = ref<{ key: string, value: number } | null>(null)
const trackEl    = ref<HTMLElement | null>(null)

// A drag moves this preview alone, the config committing on release, so the
// code and toml surfaces hold still while a stop is in hand.
function shownValue(knob: LengthKnob): number {
  const held = preview.value
  return held && held.key === knob.key ? held.value : props.valueOf(knob.key)
}

const { width: trackWidth } = useElementSize(trackEl)

let chipObserver: ResizeObserver | null = null

onBeforeUnmount(() => chipObserver?.disconnect())

// Left-to-right, each stop takes the lowest tier whose occupant sits clear of
// it, so chips step up only while they meet and drop back once clear.
const tierByKey = computed(() => {
  const stops = props.lengths
    .map(knob => ({ key: knob.key, pct: pctOf(shownValue(knob)) }))
    .sort((a, b) => a.pct - b.pct)
  const placed: Stop[] = []
  const tiers = new Map<string, number>()
  for (const stop of stops) {
    let tier = 0
    while (tier < placed.length && collides(placed[tier], stop)) tier += 1
    placed[tier] = stop
    tiers.set(stop.key, tier)
  }
  return tiers
})

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

// Mirrors the CSS `translateX` clamp, so a chip pinned at a track edge is
// judged where it renders rather than where its stop sits.
function chipCenter(stop: Stop): number {
  const track = trackWidth.value || TRACK_FALLBACK
  const width = chipWidth(stop.key)
  const x     = (stop.pct / 100) * track
  const shift = Math.max(-x, Math.min(-width / 2, track - x - width))
  return x + shift + width / 2
}

function chipWidth(key: string): number {
  return chipWidths.get(key) ?? CHIP_FALLBACK
}

function clamp(value: number): number {
  return Math.min(MAX, Math.max(MIN, Math.round(value)))
}

// Two chips collide when their rendered centers sit closer than the pair's
// mean measured width plus a clearance.
function collides(a: Stop, b: Stop): boolean {
  const clear = (chipWidth(a.key) + chipWidth(b.key)) / 2 + CLEARANCE
  return Math.abs(chipCenter(a) - chipCenter(b)) < clear
}

function commitEdit(knob: LengthKnob): void {
  if (editing.value !== knob.key) return
  const parsed = Number.parseInt(draft.value, 10)
  if (Number.isFinite(parsed)) emit('setLength', knob.key, clamp(parsed))
  editing.value = ''
}

function dragEnd(event: PointerEvent): void {
  if (drag.value?.pointerId !== event.pointerId) return
  const held = preview.value
  if (held) emit('setLength', held.key, held.value)
  preview.value = null
  drag.value    = null
  emit('dragging', '')
}

function dragMove(knob: LengthKnob, event: PointerEvent): void {
  const state = drag.value
  if (!state || state.key !== knob.key || state.pointerId !== event.pointerId) return
  if (!state.live && Math.abs(event.clientX - state.startX) < DRAG_SLOP) return
  state.live = true
  const value = clamp((event.clientX - state.originX) / state.perChar + MIN)
  preview.value = { key: knob.key, value }
  emit('preview', knob.key, value)
}

function dragStart(knob: LengthKnob, event: PointerEvent): void {
  if (editing.value === knob.key || event.button !== 0 || !trackEl.value) return
  event.preventDefault()
  const stop = event.currentTarget as HTMLElement
  const rect = trackEl.value.getBoundingClientRect()
  const unit = rect.width / SPAN
  stop.setPointerCapture(event.pointerId)
  emit('dragging', knob.key)
  drag.value = {
    key       : knob.key,
    live      : false,
    originX   : event.clientX - (clamp(props.valueOf(knob.key)) - MIN) * unit,
    perChar   : unit,
    pointerId : event.pointerId,
    startX    : event.clientX
  }
}

function ensureObserver(): ResizeObserver | null {
  if (typeof ResizeObserver === 'undefined') return null
  chipObserver ??= new ResizeObserver(entries => {
    for (const entry of entries) {
      const key = chipKeys.get(entry.target)
      if (key !== undefined) chipWidths.set(key, (entry.target as HTMLElement).offsetWidth)
    }
  })
  return chipObserver
}

function focusInput(el: unknown): void {
  if (el instanceof HTMLInputElement && el !== document.activeElement) el.select()
}

function hueOf(index: number): string {
  return `var(--prose-palette-${HUE_NAMES[index % HUE_NAMES.length]})`
}

function keyStep(knob: LengthKnob, event: KeyboardEvent): void {
  const next = targetValue(event, props.valueOf(knob.key))
  if (next === null) return
  event.preventDefault()
  emit('setLength', knob.key, clamp(next))
}

function observeChip(el: unknown, key: string): void {
  if (!(el instanceof HTMLElement) || observed.has(el)) return
  observed.add(el)
  chipKeys.set(el, key)
  ensureObserver()?.observe(el)
}

function pctOf(value: number): number {
  return ((clamp(value) - MIN) / SPAN) * 100
}

function stopStyle(index: number, knob: LengthKnob): CSSProperties {
  return {
    '--hue'  : hueOf(index),
    '--pct'  : pctOf(shownValue(knob)),
    '--tier' : tierOf(knob.key)
  }
}

function targetValue(event: KeyboardEvent, value: number): number | null {
  const step = event.shiftKey ? 10 : 1
  switch (event.key) {
    case 'ArrowDown':
    case 'ArrowLeft':  return value - step
    case 'ArrowRight':
    case 'ArrowUp':    return value + step
    case 'PageDown':   return value - 10
    case 'PageUp':     return value + 10
    case 'End':        return MAX
    case 'Home':       return MIN
    default:           return null
  }
}

function tierOf(key: string): number {
  return tierByKey.value.get(key) ?? 0
}
</script>

<template>
  <div class="ruler-head">
    <div ref="trackEl" class="ruler" role="group" aria-label="Line lengths" :style="{ '--rows': rows }">
      <div class="ruler-track" aria-hidden="true" />
      <span
        v-for="(knob, index) in lengths"
        :key="`default-${knob.key}`"
        class="ruler-default"
        aria-hidden="true"
        :style="{ left: `${pctOf(knob.default)}%`, '--hue': hueOf(index) }"
      />
      <span
        v-for="mark in MARKS"
        :key="mark"
        class="ruler-num"
        aria-hidden="true"
        :style="{ left: `${pctOf(mark)}%` }"
      >{{ mark }}</span>
      <div
        v-for="(knob, index) in lengths"
        :key="knob.key"
        class="ruler-stop"
        :style="stopStyle(index, knob)"
        :data-dragging="drag?.key === knob.key || null"
        :role="editing === knob.key ? undefined : 'slider'"
        :tabindex="editing === knob.key ? undefined : 0"
        :aria-label="`${knob.label} line length`"
        :aria-valuemin="MIN"
        :aria-valuemax="MAX"
        :aria-valuenow="shownValue(knob)"
        @pointerdown="dragStart(knob, $event)"
        @pointermove="dragMove(knob, $event)"
        @pointerup="dragEnd"
        @pointercancel="dragEnd"
        @keydown="keyStep(knob, $event)"
      >
        <span class="ruler-marker" aria-hidden="true" />
        <span
          v-if="editing !== knob.key"
          :ref="el => observeChip(el, knob.key)"
          class="ruler-chip"
          @dblclick="beginEdit(knob)"
        >
          <span class="ruler-chip-label">{{ knob.label }}</span>
          <span class="ruler-chip-value">{{ shownValue(knob) }}</span>
        </span>
        <input
          v-else
          :ref="focusInput"
          v-model="draft"
          type="number"
          class="ruler-chip ruler-chip-input"
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

<style scoped>
.ruler-head {
  display     : flex;
  flex-wrap   : wrap;
  gap         : 0.75rem 1.25rem;
  align-items : center;
}

.ruler {
  --ruler-chip-h  : 20px;
  --ruler-gap     : 5px;
  --ruler-major   : var(--vp-c-text-3);
  --ruler-minor   : var(--vp-c-divider);
  --ruler-nums-h  : 14px;
  --ruler-row     : calc(var(--ruler-chip-h) + var(--ruler-gap));
  --ruler-track-h : 16px;

  position       : relative;
  flex           : 1 1 16rem;
  min-width      : 0;
  height         : calc(var(--rows, 1) * var(--ruler-row) + var(--ruler-track-h) + var(--ruler-nums-h));
  container-type : inline-size;
  user-select    : none;
  transition     : height var(--prose-transition);
}

.ruler-track {
  position     : absolute;
  top          : calc(var(--rows, 1) * var(--ruler-row));
  transition   : top var(--prose-transition);
  right        : 0;
  left         : 0;
  height       : var(--ruler-track-h);
  border-block : 1px solid var(--ruler-minor);
  border-right : 1px solid var(--ruler-major);
  background   : repeating-linear-gradient(
                   90deg,
                   var(--ruler-major) 0 1px,
                   transparent 0 calc(100% / 3)
                 ) left top / 100% 100% no-repeat,
                 repeating-linear-gradient(
                   90deg,
                   var(--ruler-minor) 0 1px,
                   transparent 0 calc(100% / 15)
                 ) left bottom / 100% 62% no-repeat;
}

.ruler-default {
  position   : absolute;
  top        : calc(var(--rows, 1) * var(--ruler-row) + var(--ruler-track-h) / 2);
  width      : 6px;
  height     : 6px;
  transform  : translate(-3px, -3px) rotate(45deg);
  background : var(--hue);
  opacity    : 0.8;
  transition : top var(--prose-transition);
}

.ruler-num {
  position    : absolute;
  top         : calc(var(--rows, 1) * var(--ruler-row) + var(--ruler-track-h) + 3px);
  transform   : translateX(-50%);
  color       : var(--vp-c-text-3);
  font-family : var(--vp-font-family-mono);
  font-size   : 0.6rem;
  line-height : 1;
  transition  : top var(--prose-transition);
}

.ruler-num:first-of-type {
  transform : none;
}

.ruler-num:last-of-type {
  transform : translateX(-100%);
}

.ruler-stop {
  position     : absolute;
  top          : 0;
  bottom       : 0;
  left         : calc(var(--pct) * 1%);
  width        : 0;
  cursor       : ew-resize;
  touch-action : none;
  user-select  : none;
  outline      : none;
}

.ruler-stop:hover,
.ruler-stop:focus-within,
.ruler-stop[data-dragging] {
  z-index : 2;
}

.ruler-marker {
  position   : absolute;
  top        : calc((var(--rows, 1) - 1 - var(--tier)) * var(--ruler-row) + var(--ruler-chip-h) - 2px);
  bottom     : var(--ruler-nums-h);
  left       : -5px;
  width      : 10px;
  transition : top var(--prose-transition);
}

.ruler-marker::before {
  content    : '';
  position   : absolute;
  inset      : 0 4px;
  background : color-mix(in srgb, var(--hue) 70%, var(--vp-c-text-3));
  transition : background var(--prose-transition);
}

.ruler-stop:hover .ruler-marker::before,
.ruler-stop[data-dragging] .ruler-marker::before {
  background : var(--hue);
}

.ruler-chip {
  position      : absolute;
  top           : calc((var(--rows, 1) - 1 - var(--tier)) * var(--ruler-row));
  left          : 0;
  z-index       : 1;
  display       : inline-flex;
  gap           : 0.4rem;
  align-items   : center;
  height        : var(--ruler-chip-h);
  padding       : 0 7px;
  border        : 1px solid color-mix(in srgb, var(--hue) 55%, var(--vp-c-divider));
  border-radius : var(--prose-radius-sm);
  background    : var(--vp-c-bg);
  line-height   : 1;
  white-space   : nowrap;
  transform     : translateX(clamp(calc(var(--pct) * -1cqw), -50%, calc((100 - var(--pct)) * 1cqw - 100%)));
  transition    : border-color var(--prose-transition), top var(--prose-transition);
}

.ruler-stop:hover .ruler-chip,
.ruler-stop[data-dragging] .ruler-chip {
  border-color : var(--hue);
}

.ruler-stop:focus-visible .ruler-chip {
  outline        : var(--prose-focus-ring);
  outline-offset : 1px;
}

.ruler-chip-label {
  color          : var(--vp-c-text-2);
  font-family    : var(--vp-font-family-mono);
  font-size      : 0.62rem;
  letter-spacing : 0.05em;
  text-transform : uppercase;
}

.ruler-chip-value {
  color                : var(--vp-c-text-1);
  font-family          : var(--vp-font-family-mono);
  font-size            : 0.68rem;
  font-variant-numeric : tabular-nums;
}

.ruler-chip-input {
  width       : 3.4rem;
  padding     : 0 5px;
  color       : var(--vp-c-text-1);
  font-family : var(--vp-font-family-mono);
  font-size   : 0.68rem;
  text-align  : center;
  cursor      : text;
  user-select : text;
  appearance  : textfield;
}

.ruler-chip-input::-webkit-outer-spin-button,
.ruler-chip-input::-webkit-inner-spin-button {
  margin     : 0;
  appearance : none;
}

.ruler-chip-input:focus-visible {
  outline        : var(--prose-focus-ring);
  outline-offset : -1px;
}

@media (prefers-reduced-motion: reduce) {
  .ruler,
  .ruler-track,
  .ruler-default,
  .ruler-num,
  .ruler-chip,
  .ruler-marker,
  .ruler-marker::before {
    transition : none;
  }
}
</style>
