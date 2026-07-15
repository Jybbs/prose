import { fc, test } from '@fast-check/vitest'

import { randomOther } from '../../lib/sandbox/session'

afterEach(() => { vi.restoreAllMocks() })

describe('randomOther', () => {
  it('stays on the only case the pool holds', () => {
    expect(randomOther(1, 0)).toBe(0)
  })

  it.each([
    [0,    0, 1],
    [0,    2, 0],
    [0.99, 0, 2]
  ])('rolls %j against the showing case %j and lands on %j', (roll, exclude, expected) => {
    vi.spyOn(Math, 'random').mockReturnValue(roll)
    expect(randomOther(3, exclude)).toBe(expected)
  })

  test.prop([fc.integer({ min: 2, max: 20 }), fc.nat()])(
    'never lands on the case already showing',
    (count, seed) => {
      expect(randomOther(count, seed % count)).not.toBe(seed % count)
    }
  )
})
