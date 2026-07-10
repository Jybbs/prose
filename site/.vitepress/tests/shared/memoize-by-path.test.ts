import { memoizeByPath } from '../../lib/shared/memoize-by-path'

describe('memoizeByPath', () => {
  it('computes once per distinct path', () => {
    const compute = vi.fn<(dir: string) => string>(dir => dir.toUpperCase())
    const memo    = memoizeByPath(compute)
    expect([memo('a'), memo('a'), memo('b')]).toEqual(['A', 'A', 'B'])
    expect(compute).toHaveBeenCalledTimes(2)
  })

  it('caches an undefined result', () => {
    const compute = vi.fn<(dir: string) => undefined>()
    const memo    = memoizeByPath(compute)
    memo('a')
    memo('a')
    expect(compute).toHaveBeenCalledTimes(1)
  })
})
