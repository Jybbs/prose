import type { GlossaryEntry } from './glossary-data'

export function buildPhraseToSlug(source: Record<string, GlossaryEntry>): Map<string, string> {
  const out = new Map<string, string>()
  for (const [slug, entry] of Object.entries(source)) {
    register(out, slug, slug)
    for (const alias of entry.aliases ?? []) {
      register(out, alias, slug)
    }
  }
  return out
}

function register(map: Map<string, string>, phrase: string, slug: string): void {
  const existing = map.get(phrase)
  if (existing !== undefined && existing !== slug) {
    throw new Error(`Glossary phrase "${phrase}" registered against both "${existing}" and "${slug}"`)
  }
  map.set(phrase, slug)
}
