import { useTimeoutFn }    from '@vueuse/core'
import { watch, type Ref } from 'vue'

import type { LandingTypingDemoEntry, LandingTypingDemoResetRow } from '../../../lib/landing/typing-demo'
import { editPlan } from '../../../lib/landing/typing-demo-buffer'

export type Phase =
  | 'editBackspacing'
  | 'editTyping'
  | 'holdAfterReset'
  | 'holdAfterTyped'
  | 'holdAtEnd'
  | 'holdBetweenEdits'
  | 'reducedMotion'
  | 'resetBackspacing'
  | 'resetTyping'
  | 'starting'

const EDIT_BACKSPACE_MS_PER_CHAR = 70
const HOLD_AFTER_RESET_MS        = 1200
const HOLD_AT_END_MS             = 3500
const HOLD_BETWEEN_EDITS_MS      = 650
export const MAGIC_MOVE_MS       = 420
const RESET_MS_PER_STEP          = 32
const SHIFT_AFTER_TYPED_MS       = 34
const SHIFT_HOLD_MS              = 480
const TYPE_MS_PER_CHAR           = 22

interface MachineRefs {
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
  entries   : readonly LandingTypingDemoEntry[],
  resetRows : readonly LandingTypingDemoResetRow[],
  refs      : MachineRefs,
  gates     : MachineGates
) {
  const resetBackspaceSteps = Math.max(...resetRows.map(row => row.end.length))
  const resetTypeSteps      = Math.max(...resetRows.map(row => row.prelude.length))

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

  function startEditBackspace(): void {
    refs.phase.value        = 'editBackspacing'
    refs.editProgress.value = 0
    tickEditBackspace()
  }

  function tickEditBackspace(): void {
    refs.editProgress.value++
    const entry = entries[refs.entryIndex.value]
    if (refs.editProgress.value === editPlan(entry.from, entry.to).fromCore.length) {
      refs.phase.value        = 'editTyping'
      refs.editProgress.value = 0
      schedule(tickEditType, TYPE_MS_PER_CHAR)
    } else {
      schedule(tickEditBackspace, EDIT_BACKSPACE_MS_PER_CHAR)
    }
  }

  function tickEditType(): void {
    refs.editProgress.value++
    const entry = entries[refs.entryIndex.value]
    if (refs.editProgress.value < editPlan(entry.from, entry.to).toCore.length) {
      schedule(tickEditType, TYPE_MS_PER_CHAR)
    } else {
      schedule(settleTyped, SHIFT_AFTER_TYPED_MS)
    }
  }

  function settleTyped(): void {
    refs.phase.value = 'holdAfterTyped'
    shiftRight()
  }

  function shiftRight(): void {
    refs.pythonStateIndex.value = refs.entryIndex.value + 1
    schedule(settleAfterShift, SHIFT_HOLD_MS)
  }

  function settleAfterShift(): void {
    if (refs.entryIndex.value === entries.length - 1) {
      refs.phase.value = 'holdAtEnd'
      schedule(startReset, HOLD_AT_END_MS)
    } else {
      refs.phase.value = 'holdBetweenEdits'
      schedule(advanceEntry, HOLD_BETWEEN_EDITS_MS)
    }
  }

  function advanceEntry(): void {
    refs.entryIndex.value++
    startEditBackspace()
  }

  function startReset(): void {
    refs.pythonStateIndex.value = 0
    refs.phase.value            = 'resetBackspacing'
    refs.editProgress.value     = 0
    schedule(tickResetBackspace, RESET_MS_PER_STEP)
  }

  function tickResetBackspace(): void {
    refs.editProgress.value++
    if (refs.editProgress.value >= resetBackspaceSteps) {
      refs.phase.value        = 'resetTyping'
      refs.editProgress.value = 0
      schedule(tickResetType, RESET_MS_PER_STEP)
    } else {
      schedule(tickResetBackspace, RESET_MS_PER_STEP)
    }
  }

  function tickResetType(): void {
    refs.editProgress.value++
    if (refs.editProgress.value >= resetTypeSteps) {
      refs.phase.value = 'holdAfterReset'
      schedule(restart, HOLD_AFTER_RESET_MS)
    } else {
      schedule(tickResetType, RESET_MS_PER_STEP)
    }
  }

  function restart(): void {
    refs.entryIndex.value       = 0
    refs.editProgress.value     = 0
    refs.pythonStateIndex.value = 0
    startEditBackspace()
  }

  function freezeAtEnd(): void {
    const last = entries[entries.length - 1]
    refs.entryIndex.value       = entries.length - 1
    refs.editProgress.value     = editPlan(last.from, last.to).toCore.length
    refs.pythonStateIndex.value = entries.length
    refs.phase.value            = 'reducedMotion'
  }

  function replay(): void {
    stop()
    refs.entryIndex.value       = 0
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
