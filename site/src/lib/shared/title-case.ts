import * as fc from 'fast-check'

export function titleCase(slug: string): string {
  return slug.split('-').map(word => word.charAt(0).toUpperCase() + word.slice(1)).join(' ')
}

if (import.meta.vitest) {
  const { describe, expect, test } = import.meta.vitest

  describe('titleCase', () => {
    test.each([
      { name: 'capitalizes a single word',        slug: 'lint',            expected: 'Lint' },
      { name: 'splits a hyphenated slug',         slug: 'align-equals',    expected: 'Align Equals' },
      { name: 'capitalizes every segment',        slug: 'multi-word-slug', expected: 'Multi Word Slug' },
      { name: 'leaves an already-capital word',   slug: 'Lint',            expected: 'Lint' },
      { name: 'returns an empty string untouched', slug: '',               expected: '' }
    ])('$name', ({ slug, expected }) => {
      expect(titleCase(slug)).toBe(expected)
    })

    test('preserves word count, drops hyphens, and uppercases each segment head', () => {
      fc.assert(fc.property(fc.array(fc.constantFrom('align', 'equals', 'lint', 'docs'), { minLength: 1, maxLength: 5 }), (words) => {
        const out = titleCase(words.join('-')).split(' ')
        expect(out).toHaveLength(words.length)
        for (const word of out) expect(word).toMatch(/^[A-Z]/)
      }))
    })
  })
}
