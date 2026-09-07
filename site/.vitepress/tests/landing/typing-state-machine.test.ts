import type {
  LandingTypingDemoEntry, LandingTypingDemoResetRow
} from '../../lib/landing/typing-demo'
import * as stateMachine from '../../lib/landing/typing-state-machine'

const ENTRIES: readonly LandingTypingDemoEntry[] = [
  { anchor: 'a = ', from: 'false', kind: 'edit', slug: 'a', to: 'true' },
  { anchor: 'b = ', from: 'no',    kind: 'edit', slug: 'b', to: 'yes'  }
]

const RESET_ROWS: readonly LandingTypingDemoResetRow[] = [
  { anchor: 'a = ', end: 'true', prelude: 'false' },
  { anchor: 'b = ', end: 'yes',  prelude: 'no'    }
]

interface Harness {
  machine : ReturnType<typeof stateMachine.createTypingMachine>
  phases  : stateMachine.MachineState['phase'][]
  reduced : { value: boolean }
  states  : stateMachine.MachineState[]
}

const harness = (): Harness => {
  const phases  : stateMachine.MachineState['phase'][] = []
  const states  : stateMachine.MachineState[]          = []
  const reduced = { value: false }
  const machine = stateMachine.createTypingMachine({
    entries       : ENTRIES,
    onChange      : state => { phases.push(state.phase); states.push({ ...state }) },
    reducedMotion : () => reduced.value,
    resetRows     : RESET_ROWS
  })
  return { machine, phases, reduced, states }
}

beforeEach(() => {
  vi.useFakeTimers()
})

afterEach(() => {
  vi.useRealTimers()
})

describe('createTypingMachine', () => {
  it.each([
    'editBackspacing', 'editTyping', 'holdAfterTyped', 'holdBetweenEdits',
    'holdAtEnd', 'resetBackspacing', 'resetTyping', 'holdAfterReset'
  ] satisfies stateMachine.Phase[])('reaches %s in one full loop', phase => {
    const { machine, phases } = harness()
    machine.boot()
    machine.setInView(true)
    vi.advanceTimersByTime(20_000)
    expect(phases).toContain(phase)
  })

  it('parks the pending tick out of view and resumes on re-entry', () => {
    const { machine, phases } = harness()
    machine.boot()
    machine.setInView(true)
    vi.advanceTimersByTime(stateMachine.MAGIC_MOVE_MS)
    machine.setInView(false)
    const parked = phases.length
    vi.advanceTimersByTime(20_000)
    expect(phases).toHaveLength(parked)
    machine.setInView(true)
    vi.advanceTimersByTime(20_000)
    expect(phases.length).toBeGreaterThan(parked)
  })

  it('never ticks under reduced motion', () => {
    const { machine, phases, reduced } = harness()
    reduced.value = true
    machine.boot()
    machine.setInView(true)
    vi.advanceTimersByTime(20_000)
    expect(phases).toHaveLength(0)
  })

  it('freezes at the terminal state', () => {
    const { machine, states } = harness()
    machine.freezeAtEnd()
    expect(states.at(-1)).toStrictEqual({
      editProgress     : 3,
      entryIndex       : 1,
      phase            : 'reducedMotion',
      pythonStateIndex : 2
    })
  })

  it('lands replay on the frozen end after the magic move', () => {
    const { machine, states } = harness()
    machine.setInView(true)
    machine.replay()
    vi.advanceTimersByTime(stateMachine.MAGIC_MOVE_MS)
    expect(states.at(-1)?.phase).toBe('reducedMotion')
    expect(states.at(-1)?.pythonStateIndex).toBe(2)
  })

  it('drops the pending tick for good on dispose', () => {
    const { machine, phases } = harness()
    machine.boot()
    machine.setInView(true)
    machine.dispose()
    vi.advanceTimersByTime(20_000)
    machine.setInView(true)
    vi.advanceTimersByTime(20_000)
    expect(phases).toHaveLength(0)
  })
})
