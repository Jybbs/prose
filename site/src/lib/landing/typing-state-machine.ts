import { editPlan }                                 from './typing-demo-buffer'
import type { TypingDemoEntry, TypingDemoResetRow } from './typing-demo-buffer'

type Phase =
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
  entries       : readonly TypingDemoEntry[]
  onChange      : (state: Readonly<MachineState>) => void
  reducedMotion : () => boolean
  resetRows     : readonly TypingDemoResetRow[]
  timings       ?: TypingTimings
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
// state and reporting it through `onChange` for the island to draw. Leaving
// the viewport parks the pending tick, re-entering resumes it.
export function createTypingMachine(options: MachineOptions): TypingMachine {
  const { entries, onChange, reducedMotion, resetRows, timings = TYPING_TIMINGS } = options

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
    freezeAtEnd,

    boot(): void {
      schedule(restart, timings.magicMoveMs)
    },

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

if (import.meta.vitest) {
  const { afterEach, beforeEach, describe, expect, test, vi } = import.meta.vitest

  const ENTRIES: readonly TypingDemoEntry[] = [
    { anchor: 'a = ', from: 'false', kind: 'edit', slug: 'a', to: 'true' },
    { anchor: 'b = ', from: 'no',    kind: 'edit', slug: 'b', to: 'yes'  }
  ]

  const RESET_ROWS: readonly TypingDemoResetRow[] = [
    { anchor: 'a = ', end: 'true', prelude: 'false' },
    { anchor: 'b = ', end: 'yes',  prelude: 'no'    }
  ]

  interface Harness {
    machine : ReturnType<typeof createTypingMachine>
    phases  : Phase[]
    reduced : { value: boolean }
    states  : MachineState[]
  }

  const harness = (): Harness => {
    const phases  : Phase[]        = []
    const states  : MachineState[] = []
    const reduced = { value: false }
    const machine = createTypingMachine({
      entries       : ENTRIES,
      onChange      : state => { phases.push(state.phase); states.push({ ...state }) },
      reducedMotion : () => reduced.value,
      resetRows     : RESET_ROWS
    })
    return { machine, phases, reduced, states }
  }

  beforeEach(() => {
    vi.useFakeTimers()
    vi.stubGlobal('window', globalThis)
  })

  afterEach(() => {
    vi.useRealTimers()
    vi.unstubAllGlobals()
  })

  describe('createTypingMachine', () => {
    test('drives one full loop through every animated phase', () => {
      const { machine, phases } = harness()
      machine.boot()
      machine.setInView(true)
      vi.advanceTimersByTime(20_000)
      for (const phase of [
        'editBackspacing', 'editTyping', 'holdAfterTyped', 'holdBetweenEdits',
        'holdAtEnd', 'resetBackspacing', 'resetTyping', 'holdAfterReset'
      ] satisfies Phase[]) {
        expect(phases).toContain(phase)
      }
    })

    test('parks the pending tick out of view and resumes on re-entry', () => {
      const { machine, phases } = harness()
      machine.boot()
      machine.setInView(true)
      vi.advanceTimersByTime(MAGIC_MOVE_MS)
      machine.setInView(false)
      const parked = phases.length
      vi.advanceTimersByTime(20_000)
      expect(phases.length).toBe(parked)
      machine.setInView(true)
      vi.advanceTimersByTime(20_000)
      expect(phases.length).toBeGreaterThan(parked)
    })

    test('never ticks under reduced motion', () => {
      const { machine, phases, reduced } = harness()
      reduced.value = true
      machine.boot()
      machine.setInView(true)
      vi.advanceTimersByTime(20_000)
      expect(phases).toHaveLength(0)
    })

    test('freezes at the terminal state', () => {
      const { machine, states } = harness()
      machine.freezeAtEnd()
      expect(states.at(-1)).toEqual({
        editProgress     : 3,
        entryIndex       : 1,
        phase            : 'reducedMotion',
        pythonStateIndex : 2
      })
    })

    test('replay lands on the frozen end after the magic move', () => {
      const { machine, states } = harness()
      machine.setInView(true)
      machine.replay()
      vi.advanceTimersByTime(MAGIC_MOVE_MS)
      expect(states.at(-1)?.phase).toBe('reducedMotion')
      expect(states.at(-1)?.pythonStateIndex).toBe(2)
    })
  })
}
