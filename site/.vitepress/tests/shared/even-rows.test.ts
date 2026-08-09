import { evenRows } from '../../lib/shared/even-rows'

const OPTIONS = { available: 250, gap: 3, minWidth: 28 }

const sizes = (items: number, overrides: Partial<typeof OPTIONS> = {}): number[] =>
  evenRows(Array.from({ length: items }, (_, i) => i), { ...OPTIONS, ...overrides })
    .map(row => row.length)

describe('evenRows', () => {
  it('returns no rows for an empty roster', () => {
    expect(evenRows([], OPTIONS)).toEqual([])
  })

  it.each([
    [3,  [3]],
    [7,  [7]],
    [8,  [8]],
    [13, [7, 6]],
    [16, [8, 8]],
    [29, [8, 7, 7, 7]]
  ])('levels %i entries into %j', (count, expected) => {
    expect(sizes(count)).toEqual(expected)
  })

  it('keeps every entry on one row while the width allows it', () => {
    expect(sizes(8)).toEqual([8])
    expect(sizes(9)).toEqual([5, 4])
  })

  it('never leaves two rows differing by more than one entry', () => {
    for (let count = 1; count <= 40; count++) {
      const rows = sizes(count)
      expect(Math.max(...rows) - Math.min(...rows)).toBeLessThanOrEqual(1)
    }
  })

  it('preserves order across the split', () => {
    expect(evenRows(['a', 'b', 'c', 'd', 'e'], { ...OPTIONS, available: 60 }))
      .toEqual([['a', 'b'], ['c', 'd'], ['e']])
  })

  it('splits further as the floor rises', () => {
    expect(sizes(13, { minWidth: 22 })).toEqual([7, 6])
    expect(sizes(13, { minWidth: 42 })).toEqual([5, 4, 4])
  })

  it('gives one entry per row where a single entry cannot fit', () => {
    expect(sizes(4, { available: 10 })).toEqual([1, 1, 1, 1])
  })

  it('holds one row while the width is still unmeasured', () => {
    expect(sizes(29, { available: 0 })).toEqual([29])
  })
})
