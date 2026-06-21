import { precompileMagicMove } from '../lib/markdown/magic-move'

describe('precompileMagicMove', () => {
  it('precompiles each code state to keyed tokens', async () => {
    const steps = await precompileMagicMove(['x = 1', 'x = 2'])
    expect(steps).toHaveLength(2)
    expect(steps[0].tokens.length).toBeGreaterThan(0)
    expect(steps[1].tokens.length).toBeGreaterThan(0)
    expect(steps[0].tokens.every(t => typeof t.key === 'string')).toBe(true)
  })
})
