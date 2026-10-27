function familyColor(family: string | null): string {
  return family ? `var(--raised-${family}, var(--vp-c-text-1))` : 'var(--vp-c-divider)'
}

export function railPaint(families: readonly (string | null)[]): string {
  if (families.length <= 1) return familyColor(families[0] ?? null)
  return `linear-gradient(to bottom, ${families.map(familyColor).join(', ')})`
}
