import { ShikiMagicMovePrecompiled }                                          from '@shikijs/magic-move/vue'
import { useIntersectionObserver }                                            from '@vueuse/core'
import { computed, defineComponent, onMounted, onUnmounted, ref, shallowRef } from 'vue'

import { ScrollRegion } from '../../../kit'

import { data }                               from '../../../lib/landing/landing-typing-demo.data'
import { useReducedMotion }                   from '../../../lib/composables/use-reduced-motion'
import * as buffer                            from '../../../lib/landing/typing-demo-buffer'
import { createTypingMachine, MAGIC_MOVE_MS } from '../../../lib/landing/typing-state-machine'
import type { Phase }                         from '../../../lib/landing/typing-state-machine'
import { magicMoveOptions }                   from '../../../lib/markdown/magic-move-options'
import { SCROLL_STOP }                        from '../../../lib/shared/constants'

export const TypingDemo = defineComponent({
  name: 'TypingDemo',
  setup() {
    // Builds `steps` and `options` once, holding one identity for the life of the
    // component, which keeps the token keys the build assigned across every
    // typing tick.
    const morphOptions = magicMoveOptions(MAGIC_MOVE_MS)
    const morphSteps   = [...data.pythonStateSteps]

    const editProgress     = ref(0)
    const entryIndex       = ref(0)
    const phase            = ref<Phase>('starting')
    const pythonStateIndex = ref(0)

    const reducedMotion = useReducedMotion()
    const rootRef = shallowRef<HTMLElement | null>(null)

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

    return () => (
      <div
        class="relative mx-auto grid max-w-1120 grid-cols-minmax-0-95fr-minmax-1-5fr gap-18 max-desktop:grid-cols-1fr"
        ref={rootRef}
      >
        <section aria-label="prose config" class="code-panel overflow-hidden panel">
          <header class="code-panel-label">prose.toml</header>
          <ScrollRegion as="pre" class="code-panel-code code-typewriter">
            {{
              default: () => (
                <code>
                    <span class="text-text-3">{segments.value.before}</span>
                    <span class="transition-colors">{segments.value.editingLineBefore}</span>
                    <span class="transition-colors">
                      {segments.value.editing}
                      {showCaret.value ? <span aria-hidden="true" class="code-caret" /> : null}
                    </span>
                    <span class="transition-colors">{segments.value.editingLineAfter}</span>
                    <span class="text-text-3">{segments.value.after}</span>
                  </code>
              )
            }}
          </ScrollRegion>
        </section>
        <section aria-label="Python source" class="code-panel overflow-hidden panel">
          <header class="code-panel-label">app.py</header>
          <ShikiMagicMovePrecompiled
            animate={!reducedMotion.value}
            class="code-panel-code"
            options={morphOptions}
            step={pythonStateIndex.value}
            steps={morphSteps}
            {...SCROLL_STOP}
          />
        </section>
        <button
          class="absolute top-12 right-12 hidden rounded-sm border-1 border-divider bg-bg px-14 py-6 font-mono text-label tracking-label text-text-2 uppercase transition-colors hover:border-brand-1 hover:text-brand-1 motion-reduce:block"
          onClick={machine.replay}
          type="button"
        >Replay</button>
      </div>
    )
  }
})
