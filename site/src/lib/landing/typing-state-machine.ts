import { editPlan }                                 from './typing-demo-buffer'
import type { TypingDemoEntry, TypingDemoResetRow } from './typing-demo-buffer'

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

export interface MachineState {
  editProgress     : number
  entryIndex       : number
  phase            : Phase
  pythonStateIndex : number
}

export interface TypingMachine {
  boot        : () => void
  freezeAtEnd : () => void
  replay      : () => void
  setInView   : (visible: boolean) => void
}

interface MachineOptions {
  entries       : readonly TypingDemoEntry[]
  onChange      : (state: Readonly<MachineState>) => void
  reducedMotion : () => boolean
  resetRows     : readonly TypingDemoResetRow[]
}

const EDIT_BACKSPACE_MS_PER_CHAR = 70
const HOLD_AFTER_RESET_MS        = 1200
const HOLD_AT_END_MS             = 3500
const HOLD_BETWEEN_EDITS_MS      = 650
export const MAGIC_MOVE_MS       = 420
const RESET_MS_PER_STEP          = 32
const SHIFT_AFTER_TYPED_MS       = 34
const SHIFT_HOLD_MS              = 480
const TYPE_MS_PER_CHAR           = 22

// The typing loop as a plain timer-driven machine, each tick mutating the
// state and reporting it through `onChange` for the island to draw. Leaving
// the viewport parks the pending tick, re-entering resumes it.
export function createTypingMachine(options: MachineOptions): TypingMachine {
  const { entries, onChange, reducedMotion, resetRows } = options

  const resetBackspaceSteps = Math.max(...resetRows.map(row => row.end.length))
  const resetTypeSteps      = Math.max(...resetRows.map(row => row.prelude.length))

  const state: MachineState = { editProgress: 0, entryIndex: 0, phase: 'starting', pythonStateIndex: 0 }

  let inView          = false
  let pendingCallback : (() => void) | null = null
  let pendingInterval = 0
  let running         = false
  let timer           = 0

  function set(partial: Partial<MachineState>): void {
    Object.assign(state, partial)
    onChange(state)
  }

  function start(): void {
    running = true
    timer   = window.setTimeout(() => {
      running = false
      const callback  = pendingCallback
      pendingCallback = null
      callback?.()
    }, pendingInterval)
  }

  function stop(): void {
    window.clearTimeout(timer)
    running = false
  }

  function schedule(callback: () => void, ms: number): void {
    pendingCallback = callback
    pendingInterval = ms
    if (!inView || reducedMotion()) {
      stop()
      return
    }
    start()
  }

  function startEditBackspace(): void {
    set({ editProgress: 0, phase: 'editBackspacing' })
    tickEditBackspace()
  }

  function tickEditBackspace(): void {
    set({ editProgress: state.editProgress + 1 })
    const entry = entries[state.entryIndex]
    if (state.editProgress === editPlan(entry.from, entry.to).fromCore.length) {
      set({ editProgress: 0, phase: 'editTyping' })
      schedule(tickEditType, TYPE_MS_PER_CHAR)
    } else {
      schedule(tickEditBackspace, EDIT_BACKSPACE_MS_PER_CHAR)
    }
  }

  function tickEditType(): void {
    set({ editProgress: state.editProgress + 1 })
    const entry = entries[state.entryIndex]
    if (state.editProgress < editPlan(entry.from, entry.to).toCore.length) {
      schedule(tickEditType, TYPE_MS_PER_CHAR)
    } else {
      schedule(settleTyped, SHIFT_AFTER_TYPED_MS)
    }
  }

  function settleTyped(): void {
    set({ phase: 'holdAfterTyped', pythonStateIndex: state.entryIndex + 1 })
    schedule(settleAfterShift, SHIFT_HOLD_MS)
  }

  function settleAfterShift(): void {
    if (state.entryIndex === entries.length - 1) {
      set({ phase: 'holdAtEnd' })
      schedule(startReset, HOLD_AT_END_MS)
    } else {
      set({ phase: 'holdBetweenEdits' })
      schedule(advanceEntry, HOLD_BETWEEN_EDITS_MS)
    }
  }

  function advanceEntry(): void {
    set({ entryIndex: state.entryIndex + 1 })
    startEditBackspace()
  }

  function startReset(): void {
    set({ editProgress: 0, phase: 'resetBackspacing', pythonStateIndex: 0 })
    schedule(tickResetBackspace, RESET_MS_PER_STEP)
  }

  function tickResetBackspace(): void {
    set({ editProgress: state.editProgress + 1 })
    if (state.editProgress >= resetBackspaceSteps) {
      set({ editProgress: 0, phase: 'resetTyping' })
      schedule(tickResetType, RESET_MS_PER_STEP)
    } else {
      schedule(tickResetBackspace, RESET_MS_PER_STEP)
    }
  }

  function tickResetType(): void {
    set({ editProgress: state.editProgress + 1 })
    if (state.editProgress >= resetTypeSteps) {
      set({ phase: 'holdAfterReset' })
      schedule(restart, HOLD_AFTER_RESET_MS)
    } else {
      schedule(tickResetType, RESET_MS_PER_STEP)
    }
  }

  function restart(): void {
    set({ editProgress: 0, entryIndex: 0, pythonStateIndex: 0 })
    startEditBackspace()
  }

  function freezeAtEnd(): void {
    const last = entries.at(-1)!
    set({
      editProgress     : editPlan(last.from, last.to).toCore.length,
      entryIndex       : entries.length - 1,
      phase            : 'reducedMotion',
      pythonStateIndex : entries.length
    })
  }

  return {
    freezeAtEnd,

    boot(): void {
      schedule(restart, MAGIC_MOVE_MS)
    },

    replay(): void {
      stop()
      set({ editProgress: 0, entryIndex: 0, phase: 'reducedMotion', pythonStateIndex: 0 })
      schedule(freezeAtEnd, MAGIC_MOVE_MS)
    },

    setInView(visible: boolean): void {
      inView = visible
      if (reducedMotion()) return
      if (visible) {
        if (pendingCallback !== null && !running) start()
      } else {
        stop()
      }
    }
  }
}
