import * as fc from 'fast-check'

// Pure folio helpers shared by the build-time render and the island scripts,
// so this module must stay free of server-only imports.

export const compareCaseless = (a: string, b: string): number =>
  a.localeCompare(b, 'en', { sensitivity: 'base' })

export function cycleIndex(delta: number, index: number, length: number): number {
  if (length === 0) return -1
  if (index < 0) return 0
  return (((index + delta) % length) + length) % length
}

// Matches against the slug and the newline-joined alias list an element
// carries in `data-aliases`, so the filter reads what the row displays.
export function entryMatches(aliases: string, query: string, slug: string): boolean {
  const q = query.trim().toLowerCase()
  if (q === '') return true
  return slug.toLowerCase().includes(q) || aliases.toLowerCase().includes(q)
}

export function groupByInitial<T extends { initial: string }>(
  entries : readonly T[]
): [string, T[]][] {
  return [...Map.groupBy(entries, entry => entry.initial).entries()]
    .toSorted(([a], [b]) => compareCaseless(a, b))
}

if (import.meta.vitest) {
  const { describe, expect, test } = import.meta.vitest

  describe('compareCaseless', () => {
    test.each([
      { name: 'treats differing case as equal', a: 'Align', b: 'align', sign: 0  },
      { name: 'orders earlier letters first',   a: 'a',     b: 'b',     sign: -1 },
      { name: 'orders later letters last',      a: 'z',     b: 'a',     sign: 1  }
    ])('$name', ({ a, b, sign }) => {
      expect(Math.sign(compareCaseless(a, b))).toBe(sign)
    })
  })

  describe('cycleIndex', () => {
    test.each([
      { name: 'returns -1 for an empty list',       delta: 1,  index: 0,  length: 0, expected: -1 },
      { name: 'clamps a negative index to zero',    delta: 1,  index: -5, length: 4, expected: 0  },
      { name: 'advances within bounds',             delta: 1,  index: 0,  length: 4, expected: 1  },
      { name: 'wraps past the end',                 delta: 1,  index: 3,  length: 4, expected: 0  },
      { name: 'wraps below zero to the last index', delta: -1, index: 0,  length: 4, expected: 3  },
      { name: 'reduces a delta larger than length', delta: 5,  index: 0,  length: 4, expected: 1  }
    ])('$name', ({ delta, index, length, expected }) => {
      expect(cycleIndex(delta, index, length)).toBe(expected)
    })

    test('stays within [0, length) for a non-negative index', () => {
      fc.assert(fc.property(fc.integer({ min: -20, max: 20 }), fc.integer({ min: 0, max: 30 }), fc.integer({ min: 1, max: 30 }), (delta, index, length) => {
        const result = cycleIndex(delta, index, length)
        expect(result).toBeGreaterThanOrEqual(0)
        expect(result).toBeLessThan(length)
      }))
    })
  })

  describe('entryMatches', () => {
    test.each([
      { name: 'matches everything on an empty query',   aliases: 'a',       query: '',    slug: 'align', expected: true  },
      { name: 'matches everything on a blank query',    aliases: 'a',       query: '   ',  slug: 'align', expected: true  },
      { name: 'matches a slug substring caselessly',    aliases: '',        query: 'ALI', slug: 'align', expected: true  },
      { name: 'matches an alias substring caselessly',  aliases: 'Foo\nBar', query: 'bar', slug: 'align', expected: true  },
      { name: 'rejects a query in neither field',       aliases: 'foo',     query: 'zzz', slug: 'align', expected: false }
    ])('$name', ({ aliases, query, slug, expected }) => {
      expect(entryMatches(aliases, query, slug)).toBe(expected)
    })
  })

  describe('groupByInitial', () => {
    test('groups entries and sorts the initials caselessly', () => {
      const entries = [
        { initial: 'C', slug: 'cat' },
        { initial: 'a', slug: 'ant' },
        { initial: 'B', slug: 'bee' }
      ]
      const grouped = groupByInitial(entries)
      expect(grouped.map(([initial]) => initial)).toEqual(['a', 'B', 'C'])
      expect(grouped.map(([, members]) => members.length)).toEqual([1, 1, 1])
    })

    test('keeps every entry within its initial bucket', () => {
      const entries = [
        { initial: 'A', slug: 'ant' },
        { initial: 'A', slug: 'ape' },
        { initial: 'B', slug: 'bee' }
      ]
      const grouped = groupByInitial(entries)
      expect(grouped).toHaveLength(2)
      expect(grouped[0][1].map(entry => entry.slug)).toEqual(['ant', 'ape'])
    })
  })
}

