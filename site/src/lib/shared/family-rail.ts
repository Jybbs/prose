// The family-color rail paint shared by the glossary folio rows and the
// composition cards, client-safe so island scripts can import it.

const familyColor = (family: string | null): string =>
  family === null ? 'var(--sl-color-hairline)' : `var(--prose-family-${family})`

export function railPaint(families: readonly (string | null)[], direction = 'to bottom'): string {
  if (families.length <= 1) return familyColor(families[0] ?? null)
  return `linear-gradient(${direction}, ${families.map(familyColor).join(', ')})`
}

if (import.meta.vitest) {
  const { describe, expect, test } = import.meta.vitest

  describe('railPaint', () => {
    test.each([
      { name: 'an empty rail is the hairline',      families: [],                    direction: undefined, expected: 'var(--sl-color-hairline)' },
      { name: 'a lone null is the hairline',        families: [null],                direction: undefined, expected: 'var(--sl-color-hairline)' },
      { name: 'a lone family is its flat color',    families: ['lint'],              direction: undefined, expected: 'var(--prose-family-lint)' },
      { name: 'two families gradient top-to-bottom', families: ['lint', 'docs'],     direction: undefined, expected: 'linear-gradient(to bottom, var(--prose-family-lint), var(--prose-family-docs))' },
      { name: 'a null stop mixes into the gradient', families: ['lint', null, 'docs'], direction: 'to right', expected: 'linear-gradient(to right, var(--prose-family-lint), var(--sl-color-hairline), var(--prose-family-docs))' }
    ])('$name', ({ families, direction, expected }) => {
      expect(direction === undefined ? railPaint(families) : railPaint(families, direction)).toBe(expected)
    })
  })
}
