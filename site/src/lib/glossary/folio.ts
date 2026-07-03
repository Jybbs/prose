// Pure folio helpers shared by the build-time render and the island scripts,
// so this module must stay free of server-only imports.

export const compareCaseless = (a: string, b: string): number =>
  a.localeCompare(b, 'en', { sensitivity: 'base' })

export function cycleIndex(index: number, delta: number, length: number): number {
  if (length === 0) return -1
  if (index < 0) return 0
  return (((index + delta) % length) + length) % length
}

// Matches against the slug and the newline-joined alias list an element
// carries in `data-aliases`, so the filter reads what the row displays.
export function entryMatches(slug: string, aliases: string, query: string): boolean {
  const q = query.trim().toLowerCase()
  if (q === '') return true
  return slug.toLowerCase().includes(q) || aliases.toLowerCase().includes(q)
}

export function groupByInitial<T extends { initial: string }>(entries: readonly T[]): [string, T[]][] {
  return [...Map.groupBy(entries, entry => entry.initial).entries()]
    .toSorted(([a], [b]) => compareCaseless(a, b))
}

