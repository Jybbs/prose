import { codeToKeyedTokens } from '@shikijs/magic-move/core'

import { precompileMagicMove } from '../../lib/markdown/magic-move'

vi.mock('@shikijs/magic-move/core', async importOriginal => {
  const core = await importOriginal<typeof import('@shikijs/magic-move/core')>()
  return { ...core, codeToKeyedTokens: vi.fn<typeof core.codeToKeyedTokens>(core.codeToKeyedTokens) }
})

describe('precompileMagicMove', () => {
  it('precompiles each code state to keyed tokens', async () => {
    const steps = await precompileMagicMove(['x = 1', 'x = 2'])
    expect(steps).toHaveLength(2)
    expect(steps[0].tokens.length).toBeGreaterThan(0)
    expect(steps[1].tokens.length).toBeGreaterThan(0)
    expect(steps[0].tokens.every(t => typeof t.key === 'string')).toBe(true)
  })

  it('tokenizes each state with no time limit', async () => {
    await precompileMagicMove(['x = 1'])
    expect(vi.mocked(codeToKeyedTokens).mock.calls.at(-1)?.[2]).toMatchObject({ tokenizeTimeLimit: 0 })
  })
})
