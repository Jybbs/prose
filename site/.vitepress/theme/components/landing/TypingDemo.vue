<script setup lang="ts">
import { useIntersectionObserver }                       from '@vueuse/core'
import { ShikiMagicMovePrecompiled }                     from 'shiki-magic-move/vue'
import { computed, onMounted, onUnmounted, ref, watch }  from 'vue'

import { data } from '../../../data/landing-typing-demo.data'
import type { LandingTypingDemoEditEntry } from '../../../data/landing-typing-demo.data'

const BACKSPACE_MS_PER_CHAR      = 5
const EDIT_BACKSPACE_MS_PER_CHAR = 70
const EDIT_TRAVEL_MS             = 520
const HOLD_AFTER_ERASED_MS       = 1200
const HOLD_AFTER_TYPED_MS        = 3500
const HOLD_BETWEEN_EDITS_MS      = 1800
const PAUSE_AFTER_ADD_MS         = 1800
const TYPE_MS_PER_CHAR           = 22

type Phase =
  | 'backspacing'
  | 'editBackspacing'
  | 'editTraveling'
  | 'editTyping'
  | 'holdAfterErased'
  | 'holdAfterTyped'
  | 'holdBetweenEdits'
  | 'reducedMotion'
  | 'starting'
  | 'typing'

type CaretLocation = 'bottom' | 'editing' | 'none'

const charProgress     = ref(0)
const editProgress     = ref(0)
const entryIndex       = ref(0)
const phase            = ref<Phase>('starting')
const pythonStateIndex = ref(0)

const reducedMotion = ref(false)
const inView        = ref(false)
const rootRef       = ref<HTMLElement | null>(null)

const typedBlocks = computed(() =>
  data.entries
    .slice(0, entryIndex.value)
    .map(e => e.kind === 'append' ? e.block : '')
    .join('')
)

const inProgressBlock = computed(() => {
  const entry = data.entries[entryIndex.value]
  if (!entry || entry.kind !== 'append') return ''
  return entry.block.slice(0, charProgress.value)
})

function applyCompletedEdits(base: string): string {
  let text = base
  for (let i = 0; i < entryIndex.value; i++) {
    const e = data.entries[i]
    if (e.kind !== 'edit') continue
    const idx = text.indexOf(e.anchor + e.from)
    if (idx === -1) continue
    const valueStart = idx + e.anchor.length
    text = text.slice(0, valueStart) + e.to + text.slice(valueStart + e.from.length)
  }
  return text
}

const bakedBufferText = computed(() => applyCompletedEdits(data.prelude + typedBlocks.value))

interface BufferSegments {
  after             : string
  before            : string
  editing           : string
  editingLineAfter  : string
  editingLineBefore : string
}

const emptySegments: BufferSegments = {
  after             : '',
  before            : '',
  editing           : '',
  editingLineAfter  : '',
  editingLineBefore : ''
}

const bufferSegments = computed<BufferSegments>(() => {
  const entry = data.entries[entryIndex.value]
  if (!entry || entry.kind !== 'edit') {
    return { ...emptySegments, before: bakedBufferText.value }
  }
  if (phase.value === 'backspacing' || phase.value === 'holdAfterErased' || phase.value === 'starting') {
    return { ...emptySegments, before: bakedBufferText.value }
  }
  return segmentsForEdit(entry, bakedBufferText.value)
})

function segmentsForEdit(entry: LandingTypingDemoEditEntry, text: string): BufferSegments {
  const anchorIdx = text.indexOf(entry.anchor + entry.from)
  if (anchorIdx === -1) return { ...emptySegments, before: text }
  const valueStart = anchorIdx + entry.anchor.length
  const valueEnd   = valueStart + entry.from.length
  const fullBefore = text.slice(0, valueStart)
  const fullAfter  = text.slice(valueEnd)

  const lastNewline       = fullBefore.lastIndexOf('\n')
  const before            = lastNewline === -1 ? '' : fullBefore.slice(0, lastNewline + 1)
  const editingLineBefore = lastNewline === -1 ? fullBefore : fullBefore.slice(lastNewline + 1)

  const firstNewline      = fullAfter.indexOf('\n')
  const editingLineAfter  = firstNewline === -1 ? fullAfter : fullAfter.slice(0, firstNewline)
  const after             = firstNewline === -1 ? '' : fullAfter.slice(firstNewline)

  let editing: string
  if (phase.value === 'editTraveling') {
    editing = entry.from
  } else if (phase.value === 'editBackspacing') {
    editing = entry.from.slice(0, entry.from.length - editProgress.value)
  } else if (phase.value === 'editTyping') {
    editing = entry.to.slice(0, editProgress.value)
  } else {
    editing = entry.to
  }
  return { after, before, editing, editingLineAfter, editingLineBefore }
}

const caretLocation = computed<CaretLocation>(() => {
  switch (phase.value) {
    case 'editBackspacing':
    case 'editTyping':
      return 'editing'
    case 'holdAfterTyped':
    case 'holdBetweenEdits': {
      const entry = data.entries[entryIndex.value]
      return entry?.kind === 'edit' ? 'editing' : 'bottom'
    }
    case 'backspacing':
    case 'typing':
      return 'bottom'
    default:
      return 'none'
  }
})

let timer:           ReturnType<typeof setTimeout> | null = null
let pendingCallback: (() => void)                  | null = null

function schedule(fn: () => void, ms: number): void {
  if (timer !== null) clearTimeout(timer)
  pendingCallback = fn
  if (!inView.value || reducedMotion.value) {
    timer = null
    return
  }
  timer = setTimeout(() => {
    pendingCallback = null
    fn()
  }, ms)
}

function runCurrentEntry(): void {
  const entry = data.entries[entryIndex.value]
  if (entry.kind === 'append') {
    phase.value = 'typing'
    schedule(tickTyping, TYPE_MS_PER_CHAR)
  } else {
    phase.value = 'editTraveling'
    schedule(startEditBackspace, EDIT_TRAVEL_MS)
  }
}

function advanceEntry(): void {
  if (entryIndex.value < data.entries.length - 1) {
    entryIndex.value++
    charProgress.value = 0
    editProgress.value = 0
    runCurrentEntry()
  } else {
    phase.value = 'holdAfterTyped'
    schedule(startBackspacing, HOLD_AFTER_TYPED_MS)
  }
}

function tickTyping(): void {
  const entry = data.entries[entryIndex.value]
  if (entry.kind !== 'append') return
  if (charProgress.value < entry.block.length) {
    charProgress.value++
    if (charProgress.value === entry.block.length) {
      pythonStateIndex.value = entryIndex.value + 1
      schedule(advanceEntry, PAUSE_AFTER_ADD_MS)
    } else {
      schedule(tickTyping, TYPE_MS_PER_CHAR)
    }
  }
}

function startEditBackspace(): void {
  phase.value        = 'editBackspacing'
  editProgress.value = 0
  tickEditBackspace()
}

function tickEditBackspace(): void {
  const entry = data.entries[entryIndex.value]
  if (entry.kind !== 'edit') return
  if (editProgress.value < entry.from.length) {
    editProgress.value++
    if (editProgress.value === entry.from.length) {
      phase.value        = 'editTyping'
      editProgress.value = 0
      schedule(tickEditType, TYPE_MS_PER_CHAR)
    } else {
      schedule(tickEditBackspace, EDIT_BACKSPACE_MS_PER_CHAR)
    }
  }
}

function tickEditType(): void {
  const entry = data.entries[entryIndex.value]
  if (entry.kind !== 'edit') return
  if (editProgress.value < entry.to.length) {
    editProgress.value++
    if (editProgress.value === entry.to.length) {
      pythonStateIndex.value = entryIndex.value + 1
      const isLast           = entryIndex.value === data.entries.length - 1
      phase.value            = isLast ? 'holdAfterTyped' : 'holdBetweenEdits'
      schedule(isLast ? startBackspacing : advanceEntry, isLast ? HOLD_AFTER_TYPED_MS : HOLD_BETWEEN_EDITS_MS)
    } else {
      schedule(tickEditType, TYPE_MS_PER_CHAR)
    }
  }
}

function lastAppendIndex(upTo: number): number {
  let i = upTo
  while (i >= 0 && data.entries[i].kind !== 'append') i--
  return i
}

function startBackspacing(): void {
  phase.value        = 'backspacing'
  editProgress.value = 0
  const current = data.entries[entryIndex.value]
  if (current?.kind === 'edit') {
    const i = lastAppendIndex(entryIndex.value - 1)
    if (i >= 0) {
      const ap           = data.entries[i]
      entryIndex.value   = i
      charProgress.value = ap.kind === 'append' ? ap.block.length : 0
    }
  }
  tickBackspacing()
}

function tickBackspacing(): void {
  if (charProgress.value > 0) {
    charProgress.value--
    if (charProgress.value === 0) {
      const i = lastAppendIndex(entryIndex.value - 1)
      if (i >= 0) {
        const ap           = data.entries[i]
        entryIndex.value   = i
        charProgress.value = ap.kind === 'append' ? ap.block.length : 0
      } else {
        phase.value            = 'holdAfterErased'
        pythonStateIndex.value = 0
        schedule(restart, HOLD_AFTER_ERASED_MS)
        return
      }
    }
    schedule(tickBackspacing, BACKSPACE_MS_PER_CHAR)
  }
}

function restart(): void {
  entryIndex.value       = 0
  charProgress.value     = 0
  editProgress.value     = 0
  pythonStateIndex.value = 0
  runCurrentEntry()
}

function freezeAtEnd(): void {
  const lastIdx          = data.entries.length - 1
  const last             = data.entries[lastIdx]
  entryIndex.value       = lastIdx
  charProgress.value     = last.kind === 'append' ? last.block.length : 0
  editProgress.value     = last.kind === 'edit'   ? last.to.length    : 0
  pythonStateIndex.value = data.entries.length
  phase.value            = 'reducedMotion'
}

function replay(): void {
  if (timer !== null) clearTimeout(timer)
  entryIndex.value       = 0
  charProgress.value     = 0
  editProgress.value     = 0
  pythonStateIndex.value = 0
  phase.value            = 'reducedMotion'
  schedule(freezeAtEnd, 600)
}

useIntersectionObserver(
  rootRef,
  ([entry]) => {
    inView.value = entry.isIntersecting
  },
  { rootMargin: '-20% 0px -20% 0px', threshold: 0 }
)

watch(inView, (visible) => {
  if (reducedMotion.value) return
  if (visible) {
    if (pendingCallback !== null && timer === null) {
      schedule(pendingCallback, 200)
    }
  } else if (timer !== null) {
    clearTimeout(timer)
    timer = null
  }
})

onMounted(() => {
  reducedMotion.value = window.matchMedia('(prefers-reduced-motion: reduce)').matches
  if (reducedMotion.value) freezeAtEnd()
  else schedule(restart, 600)
})

onUnmounted(() => {
  if (timer !== null) clearTimeout(timer)
})
</script>

<template>
  <div ref="rootRef" class="typing-demo">
    <section class="typing-demo-panel typing-demo-config" aria-label="prose config">
      <header class="typing-demo-label">pyproject.toml</header>
      <pre class="typing-demo-config-code"><code><span class="typing-demo-config-prelude">{{ bufferSegments.before }}</span><span class="typing-demo-config-editing">{{ bufferSegments.editingLineBefore }}</span><span class="typing-demo-config-editing">{{ bufferSegments.editing }}<span v-if="caretLocation === 'editing'" class="typing-demo-caret" aria-hidden="true" /></span><span class="typing-demo-config-editing">{{ bufferSegments.editingLineAfter }}</span><span class="typing-demo-config-prelude">{{ bufferSegments.after }}</span><span class="typing-demo-config-current">{{ inProgressBlock }}<span v-if="caretLocation === 'bottom'" class="typing-demo-caret" aria-hidden="true" /></span></code></pre>
    </section>
    <section class="typing-demo-panel typing-demo-python" aria-label="Python source">
      <header class="typing-demo-label">app.py</header>
      <ShikiMagicMovePrecompiled
        class    = "typing-demo-python-code"
        :steps   = "data.pythonStateSteps"
        :step    = "pythonStateIndex"
        :animate = "!reducedMotion"
        :options = "{ duration: 600, stagger: 3 }"
      />
    </section>
    <button
      v-if="reducedMotion"
      type="button"
      class="typing-demo-replay"
      @click="replay"
    >Replay</button>
  </div>
</template>
