<script setup lang="ts">
import { ShikiMagicMovePrecompiled }                             from '@shikijs/magic-move/vue'
import { useIntersectionObserver }                               from '@vueuse/core'
import { computed, onMounted, onUnmounted, ref, useTemplateRef } from 'vue'

import { data }                               from '../../../lib/landing/landing-typing-demo.data'
import { useReducedMotion }                   from '../../../lib/composables/use-reduced-motion'
import * as buffer                            from '../../../lib/landing/typing-demo-buffer'
import { createTypingMachine, MAGIC_MOVE_MS } from '../../../lib/landing/typing-state-machine'
import type { Phase }                         from '../../../lib/landing/typing-state-machine'

const editProgress     = ref(0)
const entryIndex       = ref(0)
const phase            = ref<Phase>('starting')
const pythonStateIndex = ref(0)

const reducedMotion = useReducedMotion()
const rootRef       = useTemplateRef<HTMLElement>('root')

const machine = createTypingMachine({
  entries       : data.entries,
  onChange      : state => {
    editProgress.value     = state.editProgress
    entryIndex.value       = state.entryIndex
    phase.value            = state.phase
    pythonStateIndex.value = state.pythonStateIndex
  },
  reducedMotion : () => reducedMotion.value,
  resetRows     : data.resetRows
})

const staticText = computed(() => {
  switch (phase.value) {
    case 'holdAtEnd':
    case 'reducedMotion':
      return buffer.applyCompletedEdits(data.prelude, data.entries, data.entries.length)
    default:
      return data.prelude
  }
})

const segments = computed<buffer.BufferSegments>(() => {
  const entry = data.entries[entryIndex.value]
  switch (phase.value) {
    case 'editBackspacing':
    case 'editTyping':
    case 'holdAfterTyped':
    case 'holdBetweenEdits': {
      const text = buffer.applyCompletedEdits(data.prelude, data.entries, entryIndex.value)
      return entry
        ? buffer.segmentsForEdit(entry, text, phase.value, editProgress.value)
        : { ...buffer.EMPTY_SEGMENTS, before: text }
    }
    case 'resetBackspacing':
    case 'resetTyping':
      return {
        ...buffer.EMPTY_SEGMENTS,
        before: buffer.resetText(data.prelude, data.resetRows, phase.value, editProgress.value)
      }
    default:
      return { ...buffer.EMPTY_SEGMENTS, before: staticText.value }
  }
})

const showCaret = computed(() => {
  switch (phase.value) {
    case 'editBackspacing':
    case 'editTyping':
      return true
    default:
      return false
  }
})

useIntersectionObserver(
  rootRef,
  ([entry]) => {
    machine.setInView(entry.isIntersecting)
  },
  { rootMargin: '-20% 0px -20% 0px', threshold: 0 }
)

onMounted(() => {
  if (reducedMotion.value) machine.freezeAtEnd()
  else machine.boot()
})

onUnmounted(machine.dispose)
</script>

<template>
  <div ref="root" class="typing-demo">
    <section class="code-panel typing-demo-config panel panel-clip" aria-label="prose config">
      <header class="code-panel-label">prose.toml</header>
      <pre class="code-panel-code code-typewriter"><code><span class="typing-demo-config-prelude">{{ segments.before }}</span><span class="typing-demo-config-editing">{{ segments.editingLineBefore }}</span><span class="typing-demo-config-editing">{{ segments.editing }}<span v-if="showCaret" class="code-caret" aria-hidden="true" /></span><span class="typing-demo-config-editing">{{ segments.editingLineAfter }}</span><span class="typing-demo-config-prelude">{{ segments.after }}</span></code></pre>
    </section>
    <section class="code-panel typing-demo-python panel panel-clip" aria-label="Python source">
      <header class="code-panel-label">app.py</header>
      <ShikiMagicMovePrecompiled
        class    = "code-panel-code typing-demo-python-code"
        :steps   = "[...data.pythonStateSteps]"
        :step    = "pythonStateIndex"
        :animate = "!reducedMotion"
        :options = "{ duration: MAGIC_MOVE_MS, stagger: 3 }"
      />
    </section>
    <button
      v-if="reducedMotion"
      type="button"
      class="typing-demo-replay"
      @click="machine.replay"
    >Replay</button>
  </div>
</template>
