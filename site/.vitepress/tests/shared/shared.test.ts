import { fc, test } from '@fast-check/vitest'

import { railPaint }                      from '../../lib/shared/family-rail'
import { inlineCode }                     from '../../lib/shared/inline-code'
import { externalAttrs }                  from '../../lib/shared/links'
import { lookup }                         from '../../lib/shared/lookup'
import { formatFolio, toRoman }           from '../../lib/shared/numerals'
import { toTitleCase }                    from '../../lib/shared/title-case'
import { withFallback, withFallbackSync } from '../../lib/shared/with-fallback'

import { warnTest } from '../support'

describe('toTitleCase', () => {
  it.each([
    ['align_equals',          'Align Equals'],
    ['strip_trailing_commas', 'Strip Trailing Commas'],
    ['rules_of_the_road',     'Rules of the Road'],
    ['up_and_away',           'Up and Away'],
    ['built_with_rust',       'Built with Rust'],
    ['the_inner_of',          'The Inner Of'],
    ['alphabetize',           'Alphabetize']
  ])('titles %s as %s', (slug, expected) => {
    expect(toTitleCase(slug)).toBe(expected)
  })

  it('honors a custom separator', () => {
    expect(toTitleCase('one-two-the-end', '-')).toBe('One Two the End')
  })

  const wordArb = fc.array(fc.constantFrom(...'abcdefghij'), { minLength: 1, maxLength: 6 }).map(c => c.join(''))

  test.prop([fc.array(wordArb, { minLength: 1, maxLength: 5 })])(
    'always capitalizes the first and last word',
    (words) => {
      const out = toTitleCase(words.join('_')).split(' ')
      expect(out[0]).toMatch(/^[A-Z]/)
      expect(out.at(-1)!).toMatch(/^[A-Z]/)
    }
  )
})

describe('formatFolio', () => {
  it.each([
    [1,   '01'],
    [9,   '09'],
    [42,  '42'],
    [100, '100']
  ])('pads %i to %s', (n, expected) => {
    expect(formatFolio(n)).toBe(expected)
  })

  it('honors a custom width', () => {
    expect(formatFolio(7, 3)).toBe('007')
  })

  test.prop([fc.nat()])('round-trips through Number, never shorter than the width', (n) => {
    expect(Number(formatFolio(n))).toBe(n)
    expect(formatFolio(n).length).toBeGreaterThanOrEqual(2)
  })
})

describe('toRoman', () => {
  it.each([
    [1,    'I'],
    [4,    'IV'],
    [9,    'IX'],
    [14,   'XIV'],
    [40,   'XL'],
    [90,   'XC'],
    [400,  'CD'],
    [2024, 'MMXXIV']
  ])('renders %i as %s', (n, expected) => {
    expect(toRoman(n)).toBe(expected)
  })
})

const parseRoman = (s: string): number => {
  const val: Record<string, number> = { I: 1, V: 5, X: 10, L: 50, C: 100, D: 500, M: 1000 }
  let total = 0
  for (let i = 0; i < s.length; i++) {
    total += val[s[i + 1]] > val[s[i]] ? -val[s[i]] : val[s[i]]
  }
  return total
}

test.prop([fc.integer({ min: 1, max: 3999 })])('toRoman round-trips through a parser', (n) => {
  expect(parseRoman(toRoman(n))).toBe(n)
})

describe('inlineCode', () => {
  it.each([
    ['plain text',         'plain text'],
    ['use `prose format`', 'use <code>prose format</code>'],
    ['`a` then `b`',       '<code>a</code> then <code>b</code>']
  ])('wraps backticks in %s', (input, expected) => {
    expect(inlineCode(input)).toBe(expected)
  })
})

describe('externalAttrs', () => {
  it.each([
    ['https://example.com', { rel: 'noopener', target: '_blank' }],
    ['http://example.com',  { rel: 'noopener', target: '_blank' }],
    ['/local/path',         {}],
    [undefined,             {}]
  ])('maps %s', (href, expected) => {
    expect(externalAttrs(href)).toEqual(expected)
  })
})

describe('lookup', () => {
  const registry = { alpha: 1, beta: 2 }

  it('returns the registered value', () => {
    expect(lookup(registry, 'alpha', 'Thing')).toBe(1)
  })

  it('throws with the sorted available keys', () => {
    expect(() => lookup(registry, 'gamma', 'Thing'))
      .toThrow('Thing "gamma" not registered. Available: alpha, beta')
  })
})

describe('railPaint', () => {
  it.each([
    [[],            'var(--vp-c-divider)'],
    [[null],        'var(--vp-c-divider)'],
    [['alignment'], 'var(--prose-c-family-alignment)']
  ])('paints a single or empty rail %j', (families, expected) => {
    expect(railPaint(families)).toBe(expected)
  })

  it('builds a gradient across multiple families', () => {
    expect(railPaint(['alignment', 'ordering'])).toBe(
      'linear-gradient(to bottom, var(--prose-c-family-alignment), var(--prose-c-family-ordering))'
    )
  })

  it('honors a custom direction', () => {
    expect(railPaint(['lint', 'docs'], 'to right')).toBe(
      'linear-gradient(to right, var(--prose-c-family-lint), var(--prose-c-family-docs))'
    )
  })

  const familyArb = fc.constantFrom('alignment', 'docs', 'formatting', 'layout', 'lint', 'ordering')

  test.prop([fc.array(familyArb, { minLength: 2, maxLength: 5 })])(
    'names every family token in a multi-family gradient',
    (families) => {
      const out = railPaint(families)
      for (const family of families) expect(out).toContain(`var(--prose-c-family-${family})`)
    }
  )
})

describe('withFallbackSync', () => {
  it('returns the function result on success', () => {
    expect(withFallbackSync('demo', () => 42, 0)).toBe(42)
  })

  warnTest('returns the fallback and warns on throw', ({ warn }) => {
    expect(withFallbackSync('demo', () => { throw new Error('boom') }, 7)).toBe(7)
    expect(warn).toHaveBeenCalledOnce()
  })
})

describe('withFallback', () => {
  it('resolves the function result on success', async () => {
    await expect(withFallback('demo', () => 42, 0)).resolves.toBe(42)
  })

  warnTest('resolves the fallback and warns on throw', async ({ warn }) => {
    await expect(withFallback('demo', () => { throw new Error('boom') }, 7)).resolves.toBe(7)
    expect(warn).toHaveBeenCalledOnce()
  })
})
