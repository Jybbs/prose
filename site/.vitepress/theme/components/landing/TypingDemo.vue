<script setup lang="ts">
import { useIntersectionObserver, useMediaQuery }   from '@vueuse/core'
import { ShikiMagicMovePrecompiled }                from 'shiki-magic-move/vue'
import { computed, onMounted, ref, useTemplateRef } from 'vue'

import { data }                                                from '../../../data/landing-typing-demo.data'
import { applyCompletedEdits, EMPTY_SEGMENTS, segmentsForEdit } from './typing-demo-buffer'
import type { BufferSegments }                                  from './typing-demo-buffer'
import { MAGIC_MOVE_MS, useTypingStateMachine }                 from './typing-state-machine'
import type { Phase }                                           from './typing-state-machine'

type CaretLocation = 'bottom' | 'editing' | 'none'

const charProgress     = ref(0)
const editProgress     = ref(0)
const entryIndex       = ref(0)
const phase            = ref<Phase>('starting')
const pythonStateIndex = ref(0)

const reducedMotion = useMediaQuery('(prefers-reduced-motion: reduce)')
const inView        = ref(false)
const rootRef       = useTemplateRef<HTMLElement>('root')

const { boot, freezeAtEnd, replay } = useTypingStateMachine(
  data.entries,
  { charProgress, editProgress, entryIndex, phase, pythonStateIndex },
  { inView, reducedMotion }
)

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
