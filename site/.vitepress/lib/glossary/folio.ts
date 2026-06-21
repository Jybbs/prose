import type { RenderedGlossaryEntry } from '../../data/glossary.data'

export const compareCaseless = (a: string, b: string): number =>
  a.localeCompare(b, 'en', { sensitivity: 'base' })

export const filterEntries = (
  entries : readonly RenderedGlossaryEntry[],
  query   : string
): readonly RenderedGlossaryEntry[] => {
  const q = query.trim().toLowerCase()
  if (q === '') return entries
  return entries.filter(e =>
    e.slug.toLowerCase().includes(q) || e.aliases.some(a => a.toLowerCase().includes(q)))
}

export const groupByInitial = (
  entries: readonly RenderedGlossaryEntry[]
): [string, RenderedGlossaryEntry[]][] =>
  [...Map.groupBy(entries, e => e.initial).entries()]
    .toSorted(([a], [b]) => compareCaseless(a, b))

export function cycleIndex(index: number, delta: number, length: number): number {
  if (length === 0) return -1
  if (index < 0) return 0
  return (((index + delta) % length) + length) % length
}
