export const familyRoute    = (family: string): string => `/rules/${family}/`
export const primitiveRoute = (slug: string): string => `/primitives/${slug}`
export const ruleRoute      = (family: string, slug: string): string => `/rules/${family}/${slug}`

if (import.meta.vitest) {
  const { describe, expect, test } = import.meta.vitest

  describe('route builders', () => {
    test.each([
      { name: 'familyRoute wraps a family in the rules path', actual: familyRoute('lint'),                    expected: '/rules/lint/' },
      { name: 'primitiveRoute wraps a primitive slug',        actual: primitiveRoute('aligner'),               expected: '/primitives/aligner' },
      { name: 'ruleRoute joins family and slug',              actual: ruleRoute('alignment', 'align-equals'),  expected: '/rules/alignment/align-equals' }
    ])('$name', ({ actual, expected }) => {
      expect(actual).toBe(expected)
    })
  })
}
