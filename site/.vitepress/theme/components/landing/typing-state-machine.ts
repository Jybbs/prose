import { useTimeoutFn } from '@vueuse/core'
import { watch, type Ref } from 'vue'

import type { LandingTypingDemoEntry } from './typing-demo-fixtures'

export type Phase =
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

export const BACKSPACE_MS_PER_CHAR      = 5
export const EDIT_BACKSPACE_MS_PER_CHAR = 70
export const EDIT_TRAVEL_MS             = 520
export const HOLD_AFTER_ERASED_MS       = 1200
export const HOLD_AFTER_TYPED_MS        = 3500
export const HOLD_BETWEEN_EDITS_MS      = 1800
export const MAGIC_MOVE_MS              = 600
export const PAUSE_AFTER_ADD_MS         = 1800
export const TYPE_MS_PER_CHAR           = 22

interface MachineRefs {
  charProgress     : Ref<number>
  editProgress     : Ref<number>
  entryIndex       : Ref<number>
  phase            : Ref<Phase>
  pythonStateIndex : Ref<number>
}

interface MachineGates {
  inView        : Ref<boolean>
  reducedMotion : Ref<boolean>
}

export function useTypingStateMachine(
  entries : readonly LandingTypingDemoEntry[],
  refs    : MachineRefs,
  gates   : MachineGates
) {
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
    if (!gates.inView.value || gates.reducedMotion.value) {
      stop()
      return
    }
    start()
  }

  function runCurrentEntry(): void {
    const entry = entries[refs.entryIndex.value]
    if (entry.kind === 'append') {
      refs.phase.value = 'typing'
      schedule(tickTyping, TYPE_MS_PER_CHAR)
    } else {
      refs.phase.value = 'editTraveling'
      schedule(startEditBackspace, EDIT_TRAVEL_MS)
    }
  }

  function advanceEntry(): void {
    if (refs.entryIndex.value < entries.length - 1) {
      refs.entryIndex.value++
      refs.charProgress.value = 0
      refs.editProgress.value = 0
      runCurrentEntry()
    } else {
      refs.phase.value = 'holdAfterTyped'
      schedule(startBackspacing, HOLD_AFTER_TYPED_MS)
    }
  }

  function tickTyping(): void {
    const entry = entries[refs.entryIndex.value]
    if (entry.kind !== 'append') return
    if (refs.charProgress.value < entry.block.length) {
      refs.charProgress.value++
      if (refs.charProgress.value === entry.block.length) {
        refs.pythonStateIndex.value = refs.entryIndex.value + 1
        schedule(advanceEntry, PAUSE_AFTER_ADD_MS)
      } else {
        schedule(tickTyping, TYPE_MS_PER_CHAR)
      }
    }
  }

  function startEditBackspace(): void {
    refs.phase.value        = 'editBackspacing'
    refs.editProgress.value = 0
    tickEditBackspace()
  }

  function tickEditBackspace(): void {
    const entry = entries[refs.entryIndex.value]
    if (entry.kind !== 'edit') return
    if (refs.editProgress.value < entry.from.length) {
      refs.editProgress.value++
      if (refs.editProgress.value === entry.from.length) {
        refs.phase.value        = 'editTyping'
        refs.editProgress.value = 0
        schedule(tickEditType, TYPE_MS_PER_CHAR)
      } else {
        schedule(tickEditBackspace, EDIT_BACKSPACE_MS_PER_CHAR)
      }
    }
  }

  function tickEditType(): void {
    const entry = entries[refs.entryIndex.value]
    if (entry.kind !== 'edit') return
    if (refs.editProgress.value < entry.to.length) {
      refs.editProgress.value++
      if (refs.editProgress.value === entry.to.length) {
        refs.pythonStateIndex.value = refs.entryIndex.value + 1
        const isLast                = refs.entryIndex.value === entries.length - 1
        refs.phase.value            = isLast ? 'holdAfterTyped' : 'holdBetweenEdits'
        schedule(isLast ? startBackspacing : advanceEntry, isLast ? HOLD_AFTER_TYPED_MS : HOLD_BETWEEN_EDITS_MS)
      } else {
        schedule(tickEditType, TYPE_MS_PER_CHAR)
      }
    }
  }

  function lastAppendIndex(upTo: number): number {
    let i = upTo
    while (i >= 0 && entries[i].kind !== 'append') i--
    return i
  }

  function startBackspacing(): void {
    refs.phase.value        = 'backspacing'
    refs.editProgress.value = 0
    const current = entries[refs.entryIndex.value]
    if (current?.kind === 'edit') {
      const i = lastAppendIndex(refs.entryIndex.value - 1)
      if (i >= 0) {
        const ap                = entries[i]
        refs.entryIndex.value   = i
        refs.charProgress.value = ap.kind === 'append' ? ap.block.length : 0
      }
    }
    tickBackspacing()
  }

  function tickBackspacing(): void {
    if (refs.charProgress.value > 0) {
      refs.charProgress.value--
      if (refs.charProgress.value === 0) {
        const i = lastAppendIndex(refs.entryIndex.value - 1)
        if (i >= 0) {
          const ap                = entries[i]
          refs.entryIndex.value   = i
          refs.charProgress.value = ap.kind === 'append' ? ap.block.length : 0
        } else {
          refs.phase.value            = 'holdAfterErased'
          refs.pythonStateIndex.value = 0
          schedule(restart, HOLD_AFTER_ERASED_MS)
          return
        }
      }
      schedule(tickBackspacing, BACKSPACE_MS_PER_CHAR)
    }
  }

  function restart(): void {
    refs.entryIndex.value       = 0
    refs.charProgress.value     = 0
    refs.editProgress.value     = 0
    refs.pythonStateIndex.value = 0
    runCurrentEntry()
  }

  function freezeAtEnd(): void {
    const lastIdx               = entries.length - 1
    const last                  = entries[lastIdx]
    refs.entryIndex.value       = lastIdx
    refs.charProgress.value     = last.kind === 'append' ? last.block.length : 0
    refs.editProgress.value     = last.kind === 'edit'   ? last.to.length    : 0
    refs.pythonStateIndex.value = entries.length
    refs.phase.value            = 'reducedMotion'
  }

  function replay(): void {
    stop()
    refs.entryIndex.value       = 0
    refs.charProgress.value     = 0
    refs.editProgress.value     = 0
    refs.pythonStateIndex.value = 0
    refs.phase.value            = 'reducedMotion'
    schedule(freezeAtEnd, MAGIC_MOVE_MS)
  }

  function boot(): void {
    schedule(restart, MAGIC_MOVE_MS)
  }

  watch(gates.inView, (visible) => {
    if (gates.reducedMotion.value) return
    if (visible) {
      if (pendingCallback !== null && !isPending.value) start()
    } else {
      stop()
    }
  })

  return { boot, freezeAtEnd, replay }
}
