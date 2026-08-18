// @vitest-environment happy-dom
import { useMagicMove }         from '../../lib/composables/use-magic-move'
import { magicMoveWatchdogMs }  from '../../lib/markdown/magic-move-options'
import { ruleDrawMs }           from '../../lib/shared/paint'

const commits: string[] = []
const retained = { current: { tokens: [{ key: 'k', content: 'x' }] }, previous: { tokens: [{ key: 'j', content: 'y' }] } }
let resets = 0

vi.mock('@shikijs/magic-move/vue', () => ({
  ShikiMagicMovePrecompiled: { name: 'ShikiMagicMovePrecompiled' }
}))

vi.mock('../../lib/markdown/magic-move', () => ({
  magicMoveMachine: () => Promise.resolve({
    commit : (code: string) => {
      commits.push(code)
      return retained
    },
    reset  : () => { resets += 1 }
  })
}))

describe('useMagicMove', () => {
  beforeEach(() => {
    commits.length = 0
    resets         = 0
  })

  it('tokenizes only the incoming state when stepping forward', async () => {
    const { precompile } = useMagicMove()

    await precompile('a', 'b')
    expect(commits).toEqual(['a', 'b'])

    // The machine already holds `b`, so stepping from it commits once.
    await precompile('b', 'c')
    expect(commits).toEqual(['a', 'b', 'c'])
    expect(resets).toBe(1)
  })

  it('rebases when a superseded morph left the machine ahead of the surface', async () => {
    const { precompile } = useMagicMove()
    await precompile('a', 'b')
    commits.length = 0
    resets         = 0

    // The surface still shows `a`, so `b` is not the state to step from.
    await precompile('a', 'c')
    expect(commits).toEqual(['a', 'c'])
    expect(resets).toBe(1)
  })

  it('copies each step out of the machine rather than sharing its state', async () => {
    const { precompile }      = useMagicMove()
    const { steps: [from, to] } = await precompile('a', 'b')

    // The panel assigns `key` onto the tokens it is handed, so a shared object
    // would let that write reach back into the machine's retained state.
    expect(to).not.toBe(retained.current)
    expect(to.tokens[0]).not.toBe(retained.current.tokens[0])
    expect(from.tokens[0]).not.toBe(retained.previous.tokens[0])
    expect(to.tokens[0]).toEqual(retained.current.tokens[0])

    to.tokens[0].key = 'rewritten'
    expect(retained.current.tokens[0].key).toBe('k')
  })

  it('resolves the panel and the draw duration on first use', async () => {
    const { panel, precompile, watchdogMs } = useMagicMove()
    expect(panel.value).toBeNull()
    expect(watchdogMs.value).toBe(magicMoveWatchdogMs(0))

    await precompile('a', 'b')
    expect(panel.value).not.toBeNull()
    expect(watchdogMs.value).toBe(magicMoveWatchdogMs(ruleDrawMs()))
  })
})
