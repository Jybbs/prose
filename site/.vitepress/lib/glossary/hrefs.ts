import type { GlossaryEntry } from './entries'

export function entryHref(
  slug  : string,
  entry : GlossaryEntry,
  rules : ReadonlyMap<string, { href: string }>
): string | undefined {
  if (entry.rule !== undefined) {
    const href = rules.get(entry.rule)?.href
    if (href === undefined) throw new Error(`Glossary "${slug}" names unknown rule "${entry.rule}"`)
    return href
  }
  if (entry.href?.startsWith('/rules/')) {
    throw new Error(`Glossary "${slug}" hand-writes a rule URL, use the rule field instead`)
  }
  return entry.href
}

export function glossaryHrefs(
  source : Record<string, GlossaryEntry>,
  rules  : ReadonlyMap<string, { href: string }>
): Map<string, string> {
  const out = new Map<string, string>()
  for (const [slug, entry] of Object.entries(source)) {
    const href = entryHref(slug, entry, rules)
    if (href !== undefined) out.set(slug, href)
  }
  return out
}
