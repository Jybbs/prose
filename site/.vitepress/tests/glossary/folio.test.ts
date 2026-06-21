import { fc, test } from '@fast-check/vitest'

import type { RenderedGlossaryEntry }                                 from '../../data/glossary.data'
import { compareCaseless, cycleIndex, filterEntries, groupByInitial } from '../../lib/glossary/folio'

const entry = (
  slug: string, overrides: Partial<RenderedGlossaryEntry> = {}
): RenderedGlossaryEntry => ({
  aliases        : [],
  definitionHtml : '',
  families       : ['engine'],
  initial        : slug[0]?.toUpperCase() ?? '#',
  primaryFamily  : 'engine',
  slug,
  ...overrides
})

const lowerArb = fc.array(fc.constantFrom(...'abcdefghijklmnopqrstuvwxyz'), { minLength: 1, maxLength: 10 })
  .map(chars => chars.join(''))

const fcEntry = fc.record({ aliases: fc.array(lowerArb, { maxLength: 2 }), slug: lowerArb })
  .map(({ aliases, slug }) => entry(slug, { aliases }))

describe('filterEntries', () => {
  const entries = [
    entry('align-equals', { aliases: ['equals alignment'] }),
    entry('alphabetize'),
    entry('strip-trailing-commas')
  ]

  it('returns every entry untouched for a blank query', () => {
    expect(filterEntries(entries, '   ')).toBe(entries)
  })

  it('matches on the slug, case-insensitively', () => {
    expect(filterEntries(entries, 'ALPHA').map(e => e.slug)).toEqual(['alphabetize'])
  })

  it('matches on an alias', () => {
    expect(filterEntries(entries, 'equals alignment').map(e => e.slug)).toEqual(['align-equals'])
  })

  it('returns nothing when neither slug nor alias matches', () => {
    expect(filterEntries(entries, 'nomatch')).toEqual([])
  })

  test.prop([fc.array(fcEntry, { maxLength: 30 }), fc.string()])(
    'returns an order-preserving subset of the input',
    (pool, query) => {
      const out = filterEntries(pool, query)
      expect(out.length).toBeLessThanOrEqual(pool.length)
      expect(pool.filter(e => out.includes(e))).toEqual([...out])
    }
  )

  test.prop([fc.array(fcEntry, { maxLength: 10 }), fc.constantFrom('', '   ', '\t')])(
    'returns the input by reference for a blank query',
    (pool, blank) => {
      expect(filterEntries(pool, blank)).toBe(pool)
    }
  )
})

describe('groupByInitial', () => {
  it('buckets by the precomputed initial, the buckets sorted, each group in input order', () => {
    const entries = [
      entry('beta',  { initial: 'B' }),
      entry('alpha', { initial: 'A' }),
      entry('apple', { initial: 'A' })
    ]
    expect(groupByInitial(entries).map(([letter, es]) => [letter, es.map(e => e.slug)])).toEqual([
      ['A', ['alpha', 'apple']],
      ['B', ['beta']]
    ])
  })

  test.prop([fc.array(fcEntry, { maxLength: 30 })])(
    'partitions every entry into sorted buckets, dropping none',
    (pool) => {
      const groups = groupByInitial(pool)
      const flat   = groups.flatMap(([, es]) => es)
      expect(flat.length).toBe(pool.length)
      expect(new Set(flat)).toEqual(new Set(pool))
      const letters = groups.map(([letter]) => letter)
      expect(letters).toEqual([...letters].toSorted((a, b) => compareCaseless(a, b)))
    }
  )
})

describe('cycleIndex', () => {
  it.each([
    [0,   1, 3, 1],
    [2,   1, 3, 0],
    [0,  -1, 3, 2],
    [-1,  1, 3, 0],
    [-1, -1, 3, 0]
  ])('cycles index %i by %i over length %i to %i', (index, delta, length, expected) => {
    expect(cycleIndex(index, delta, length)).toBe(expected)
  })

  it('returns -1 for an empty pool', () => {
    expect(cycleIndex(0, 1, 0)).toBe(-1)
  })

  test.prop([fc.nat(50), fc.integer({ min: -5, max: 5 }), fc.integer({ min: 1, max: 50 })])(
    'always lands within the pool bounds',
    (index, delta, length) => {
      const idx = cycleIndex(index, delta, length)
      expect(idx).toBeGreaterThanOrEqual(0)
      expect(idx).toBeLessThan(length)
    }
  )

  test.prop([fc.integer({ min: 0, max: 49 }), fc.integer({ min: 1, max: 50 })])(
    'forward then backward returns to an in-range start',
    (index, length) => {
      fc.pre(index < length)
      expect(cycleIndex(cycleIndex(index, 1, length), -1, length)).toBe(index)
    }
  )
})

describe('compareCaseless', () => {
  const wordArb = fc.array(fc.constantFrom(...'abcdABCD '), { maxLength: 8 }).map(chars => chars.join(''))

  it('ignores case', () => {
    expect(compareCaseless('Align', 'align')).toBe(0)
  })

  test.prop([wordArb, wordArb])('is antisymmetric under sign', (a, b) => {
    const ba = Math.sign(compareCaseless(b, a))
    expect(Math.sign(compareCaseless(a, b))).toBe(ba === 0 ? 0 : -ba)
  })

  test.prop([wordArb])('is reflexive', (a) => {
    expect(compareCaseless(a, a)).toBe(0)
  })
})
