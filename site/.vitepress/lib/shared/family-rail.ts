function familyColor(family: string | null): string {
  return family ? `var(--prose-c-family-${family})` : 'var(--vp-c-divider)'
}

export function railPaint(families: readonly (string | null)[], direction = 'to bottom'): string {
  if (families.length <= 1) return familyColor(families[0] ?? null)
  return `linear-gradient(${direction}, ${families.map(familyColor).join(', ')})`
}
