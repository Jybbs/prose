<script setup lang="ts">
import { useIntersectionObserver, useMediaQuery, useTimeoutFn }  from '@vueuse/core'
import { ShikiMagicMovePrecompiled }                             from 'shiki-magic-move/vue'
import { computed, onMounted, ref, useTemplateRef, watch }       from 'vue'

import { data }                                                from '../../../data/landing-typing-demo.data'
import { applyCompletedEdits, EMPTY_SEGMENTS, segmentsForEdit } from '../../../lib/markdown/typing-demo-buffer'
import type { BufferSegments }                                  from '../../../lib/markdown/typing-demo-buffer'

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

const reducedMotion = useMediaQuery('(prefers-reduced-motion: reduce)')
const inView        = ref(false)
const rootRef       = useTemplateRef<HTMLElement>('root')

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

const bakedBufferText = computed(() => applyCompletedEdits(data.prelude + typedBlocks.value, data.entries, entryIndex.value))

const bufferSegments = computed<BufferSegments>(() => {
  const entry = data.entries[entryIndex.value]
  if (!entry || entry.kind !== 'edit') {
    return { ...EMPTY_SEGMENTS, before: bakedBufferText.value }
  }
  if (phase.value === 'backspacing' || phase.value === 'holdAfterErased' || phase.value === 'starting') {
    return { ...EMPTY_SEGMENTS, before: bakedBufferText.value }
  }
  return segmentsForEdit(entry, bakedBufferText.value, phase.value, editProgress.value)
})

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

let pendingCallback: (() => void) | null = null
let pendingInterval = 0

const { start, stop, isPending } = useTimeoutFn(() => {
  const fn = pendingCallback
  pendingCallback = null
  fn?.()
}, () => pendingInterval, { immediate: false })

function schedule(fn: () => void, ms: number): void {
  pendingCallback = fn
  pendingInterval = ms
  if (!inView.value || reducedMotion.value) {
    stop()
    return
  }
  start()
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
  stop()
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
    if (pendingCallback !== null && !isPending.value) start()
  } else {
    stop()
  }
})

onMounted(() => {
  if (reducedMotion.value) freezeAtEnd()
  else schedule(restart, 600)
})
</script>

<template>
  <div ref="root" class="typing-demo">
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
