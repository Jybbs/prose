// The family-color rail paint shared by the glossary folio rows and the
// composition cards, client-safe so island scripts can import it.

const familyColor = (family: string | null): string =>
  family === null ? 'var(--sl-color-hairline)' : `var(--prose-family-${family})`

export function railPaint(families: readonly (string | null)[], direction = 'to bottom'): string {
  if (families.length <= 1) return familyColor(families[0] ?? null)
  return `linear-gradient(${direction}, ${families.map(familyColor).join(', ')})`
}
