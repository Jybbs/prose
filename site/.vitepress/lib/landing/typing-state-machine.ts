import { editPlan } from './typing-demo-buffer'

import type { LandingTypingDemoEntry, LandingTypingDemoResetRow } from './typing-demo'

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

interface TypingMachine {
  boot        : () => void
  dispose     : () => void
  freezeAtEnd : () => void
  replay      : () => void
  setInView   : (visible: boolean) => void
}

interface TypingTimings {
  editBackspaceMsPerChar : number
  holdAfterResetMs       : number
  holdAtEndMs            : number
  holdBetweenEditsMs     : number
  magicMoveMs            : number
  resetMsPerStep         : number
  shiftAfterTypedMs      : number
  shiftHoldMs            : number
  typeMsPerChar          : number
}

interface MachineOptions {
  entries       : readonly LandingTypingDemoEntry[]
  onChange      : (state: Readonly<MachineState>) => void
  reducedMotion : () => boolean
  resetRows     : readonly LandingTypingDemoResetRow[]
  timings      ?: TypingTimings
}

export const MAGIC_MOVE_MS = 420

const TYPING_TIMINGS: TypingTimings = {
  editBackspaceMsPerChar : 70,
  holdAfterResetMs       : 1200,
  holdAtEndMs            : 3500,
  holdBetweenEditsMs     : 650,
  magicMoveMs            : MAGIC_MOVE_MS,
  resetMsPerStep         : 32,
  shiftAfterTypedMs      : 34,
  shiftHoldMs            : 480,
  typeMsPerChar          : 22
}

// The typing loop as a plain timer-driven machine, each tick mutating the
// state and reporting it through `onChange` for the component to draw.
// Leaving the viewport parks the pending tick, re-entering resumes it, and
// `dispose` drops it for good when the component unmounts.
export function createTypingMachine(options: MachineOptions): TypingMachine {
  const { entries, onChange, reducedMotion, resetRows, timings = TYPING_TIMINGS } = options

  const resetBackspaceSteps = Math.max(...resetRows.map(row => row.end.length))
  const resetTypeSteps      = Math.max(...resetRows.map(row => row.prelude.length))

  const state: MachineState = {
    editProgress     : 0,
    entryIndex       : 0,
    phase            : 'starting',
    pythonStateIndex : 0
  }

  let inView          = false
  let pendingCallback : (() => void) | null = null
  let pendingInterval = 0
  let running         = false
  let timer           : ReturnType<typeof setTimeout> | undefined

  function set(partial: Partial<MachineState>): void {
    Object.assign(state, partial)
    onChange(state)
  }

  function start(): void {
    running = true
    timer   = setTimeout(() => {
      running = false
      const callback  = pendingCallback
      pendingCallback = null
      callback?.()
    }, pendingInterval)
  }

  function stop(): void {
    clearTimeout(timer)
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
      schedule(tickEditType, timings.typeMsPerChar)
    } else {
      schedule(tickEditBackspace, timings.editBackspaceMsPerChar)
    }
  }

  function tickEditType(): void {
    set({ editProgress: state.editProgress + 1 })
    const entry = entries[state.entryIndex]
    if (state.editProgress < editPlan(entry.from, entry.to).toCore.length) {
      schedule(tickEditType, timings.typeMsPerChar)
    } else {
      schedule(settleTyped, timings.shiftAfterTypedMs)
    }
  }

  function settleTyped(): void {
    set({ phase: 'holdAfterTyped', pythonStateIndex: state.entryIndex + 1 })
    schedule(settleAfterShift, timings.shiftHoldMs)
  }

  function settleAfterShift(): void {
    if (state.entryIndex === entries.length - 1) {
      set({ phase: 'holdAtEnd' })
      schedule(startReset, timings.holdAtEndMs)
    } else {
      set({ phase: 'holdBetweenEdits' })
      schedule(advanceEntry, timings.holdBetweenEditsMs)
    }
  }

  function advanceEntry(): void {
    set({ entryIndex: state.entryIndex + 1 })
    startEditBackspace()
  }

  function startReset(): void {
    set({ editProgress: 0, phase: 'resetBackspacing', pythonStateIndex: 0 })
    schedule(tickResetBackspace, timings.resetMsPerStep)
  }

  function tickResetBackspace(): void {
    set({ editProgress: state.editProgress + 1 })
    if (state.editProgress >= resetBackspaceSteps) {
      set({ editProgress: 0, phase: 'resetTyping' })
      schedule(tickResetType, timings.resetMsPerStep)
    } else {
      schedule(tickResetBackspace, timings.resetMsPerStep)
    }
  }

  function tickResetType(): void {
    set({ editProgress: state.editProgress + 1 })
    if (state.editProgress >= resetTypeSteps) {
      set({ phase: 'holdAfterReset' })
      schedule(restart, timings.holdAfterResetMs)
    } else {
      schedule(tickResetType, timings.resetMsPerStep)
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
    boot(): void {
      schedule(restart, timings.magicMoveMs)
    },

    dispose(): void {
      stop()
      pendingCallback = null
    },

    freezeAtEnd,

    replay(): void {
      stop()
      set({ editProgress: 0, entryIndex: 0, phase: 'reducedMotion', pythonStateIndex: 0 })
      schedule(freezeAtEnd, timings.magicMoveMs)
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
