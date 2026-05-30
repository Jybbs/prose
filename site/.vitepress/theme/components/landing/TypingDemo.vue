<script setup lang="ts">
import { useIntersectionObserver, useMediaQuery }   from '@vueuse/core'
import { ShikiMagicMovePrecompiled }                from 'shiki-magic-move/vue'
import { computed, onMounted, ref, useTemplateRef } from 'vue'

import { data }                                              from '../../../data/landing-typing-demo.data'
import { applyCompletedEdits, EMPTY_SEGMENTS, resetText, segmentsForEdit } from './typing-demo-buffer'
import type { BufferSegments }                  from './typing-demo-buffer'
import { MAGIC_MOVE_MS, useTypingStateMachine } from './typing-state-machine'
import type { Phase }                           from './typing-state-machine'

const editProgress     = ref(0)
const entryIndex       = ref(0)
const phase            = ref<Phase>('starting')
const pythonStateIndex = ref(0)

const reducedMotion = useMediaQuery('(prefers-reduced-motion: reduce)')
const inView        = ref(false)
const rootRef       = useTemplateRef<HTMLElement>('root')

const { boot, freezeAtEnd, replay } = useTypingStateMachine(
  data.entries,
  data.resetRows,
  { editProgress, entryIndex, phase, pythonStateIndex },
  { inView, reducedMotion }
)

const staticText = computed(() => {
  switch (phase.value) {
    case 'holdAtEnd':
    case 'reducedMotion':
      return applyCompletedEdits(data.prelude, data.entries, data.entries.length)
    default:
      return data.prelude
  }
})

const segments = computed<BufferSegments>(() => {
  const entry = data.entries[entryIndex.value]
  switch (phase.value) {
    case 'editBackspacing':
    case 'editTyping':
    case 'holdAfterTyped':
    case 'holdBetweenEdits': {
      const text = applyCompletedEdits(data.prelude, data.entries, entryIndex.value)
      return entry
        ? segmentsForEdit(entry, text, phase.value, editProgress.value)
        : { ...EMPTY_SEGMENTS, before: text }
    }
    case 'resetBackspacing':
    case 'resetTyping':
      return {
        ...EMPTY_SEGMENTS,
        before: resetText(data.prelude, data.resetRows, phase.value, editProgress.value)
      }
    default:
      return { ...EMPTY_SEGMENTS, before: staticText.value }
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
    inView.value = entry.isIntersecting
  },
  { rootMargin: '-20% 0px -20% 0px', threshold: 0 }
)

onMounted(() => {
  if (reducedMotion.value) freezeAtEnd()
  else boot()
})
</script>

<template>
  <div ref="root" class="typing-demo">
    <section class="typing-demo-panel typing-demo-config" aria-label="prose config">
      <header class="typing-demo-label">prose.toml</header>
      <pre class="typing-demo-config-code"><code><span class="typing-demo-config-prelude">{{ segments.before }}</span><span class="typing-demo-config-editing">{{ segments.editingLineBefore }}</span><span class="typing-demo-config-editing">{{ segments.editing }}<span v-if="showCaret" class="typing-demo-caret" aria-hidden="true" /></span><span class="typing-demo-config-editing">{{ segments.editingLineAfter }}</span><span class="typing-demo-config-prelude">{{ segments.after }}</span></code></pre>
    </section>
    <section class="typing-demo-panel typing-demo-python" aria-label="Python source">
      <header class="typing-demo-label">app.py</header>
      <ShikiMagicMovePrecompiled
        class    = "typing-demo-python-code"
        :steps   = "data.pythonStateSteps"
        :step    = "pythonStateIndex"
        :animate = "!reducedMotion"
        :options = "{ duration: MAGIC_MOVE_MS, stagger: 3 }"
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
