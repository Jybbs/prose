import { getCollection } from 'astro:content'

import { familyIndex, familyPage }       from './sections'
import { categoryOf, isFamily }          from '../shared/registries'
import type { RuleCategory, RuleFamily } from '../shared/registries'
import { ruleRoute }                     from '../shared/routes'

export interface DiscoveredRule {
  badge    : string
  caption  : string
  category : RuleCategory
  family   : RuleFamily
  href     : string
  related  : readonly string[]
  slug     : string
}

let cachedBadges : Promise<ReadonlyMap<string, string>> | null = null
let cachedRules  : Promise<DiscoveredRule[]> | null            = null

// The rule roster the cards, chips, and index surfaces render, read off the
// `docs` collection's rule pages with each family's badge taken from its index
// page, sorted by slug and derived once per build.
export function discoveredRules(): Promise<DiscoveredRule[]> {
  return (cachedRules ??= deriveRules())
}

// One badge per family, keyed as plain strings so glossary families resolve
// through the same lookup without a cast.
export function familyBadges(): Promise<ReadonlyMap<string, string>> {
  return (cachedBadges ??= discoveredRules().then(
    rules => new Map(rules.map(rule => [rule.family, rule.badge]))
  ))
}

async function deriveRules(): Promise<DiscoveredRule[]> {
  const docs   = await getCollection('docs')
  const badges = new Map<string, string>()
  for (const entry of docs) {
    const family = familyIndex(entry.id)
    if (family !== undefined) badges.set(family, entry.data.badge ?? '')
  }

  const rules: DiscoveredRule[] = []
  for (const entry of docs) {
    const page = familyPage(entry.id)
    if (page === undefined || !isFamily(page.family)) continue
    const { family, slug } = page as { family: RuleFamily, slug: string }
    rules.push({
      badge    : badges.get(family) ?? '',
      caption  : entry.data.caption ?? '',
      category : categoryOf(family),
      family,
      href     : ruleRoute(family, slug),
      related  : entry.data.related ?? [],
      slug
    })
  }
  return rules.sort((a, b) => a.slug.localeCompare(b.slug))
}
