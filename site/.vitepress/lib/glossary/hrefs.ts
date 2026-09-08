import type { DiscoveredRule } from '../rules/discovery'
import type { GlossaryEntry }  from './entries'

export type GlossaryRule = Pick<DiscoveredRule, 'family' | 'href'>

export function entryHref(
  slug   : string,
  entry  : GlossaryEntry,
  rule  ?: GlossaryRule
): string | undefined {
  if (rule !== undefined) return rule.href
  if (entry.href?.startsWith('/rules/')) {
    throw new Error(`Glossary "${slug}" hand-writes a rule URL, use the rule field instead`)
  }
  return entry.href
}

export function entryRule(
  slug  : string,
  entry : GlossaryEntry,
  rules : ReadonlyMap<string, GlossaryRule>
): GlossaryRule | undefined {
  if (entry.rule === undefined) return undefined
  const rule = rules.get(entry.rule)
  if (rule === undefined) throw new Error(`Glossary "${slug}" names unknown rule "${entry.rule}"`)
  return rule
}

export function glossaryHrefs(
  source : Record<string, GlossaryEntry>,
  rules  : ReadonlyMap<string, GlossaryRule>
): Map<string, string> {
  const out = new Map<string, string>()
  for (const [slug, entry] of Object.entries(source)) {
    const href = entryHref(slug, entry, entryRule(slug, entry, rules))
    if (href !== undefined) out.set(slug, href)
  }
  return out
}
