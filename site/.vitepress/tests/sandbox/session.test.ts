// @vitest-environment happy-dom
import { fc, test } from '@fast-check/vitest'

import { decodeShare }          from '../../lib/sandbox/share-link'
import { randomOther, seedUrl } from '../../lib/sandbox/session'

afterEach(() => {
  vi.restoreAllMocks()
  vi.unstubAllGlobals()
})

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

describe('seedUrl', () => {
  it('opens the sandbox on a payload restoring the seeded source and config', async () => {
    const url = await seedUrl('code-line-length = 40\n', 'x = 1\n') ?? ''
    expect(url).toMatch(/^\/sandbox\/#1\./)
    expect(await decodeShare(url.slice('/sandbox/'.length)))
      .toEqual({ configToml: 'code-line-length = 40\n', source: 'x = 1\n' })
  })

  it('yields no link where the platform lacks the compression codec', async () => {
    // oxlint-disable-next-line unicorn/no-useless-undefined -- `null` clears the typeof guard
    vi.stubGlobal('CompressionStream', undefined)
    expect(await seedUrl('', 'x = 1\n')).toBeNull()
  })
})
