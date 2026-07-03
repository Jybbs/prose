import { getCollection } from 'astro:content'

import { familyIndex, familyPage }       from './sections'
import { lazy }                          from '../../shared/lazy'
import { categoryOf, isFamily }          from '../../shared/registries'
import type { RuleCategory, RuleFamily } from '../../shared/registries'
import { required }                      from '../../shared/required'
import { ruleRoute }                     from '../../shared/routes'

export interface DiscoveredRule {
  badge    : string
  caption  : string
  category : RuleCategory
  family   : RuleFamily
  href     : string
  related  : readonly string[]
  slug     : string
}

// The rule roster the cards, chips, and index surfaces render, read off the
// `docs` collection's rule pages with each family's badge taken from its index
// page, sorted by slug and derived once per build.
export const discoveredRules = lazy(deriveRules)

// One badge per family, keyed as plain strings so glossary families resolve
// through the same lookup without a cast.
export const familyBadges = lazy(() =>
  discoveredRules().then(rules => new Map(rules.map(rule => [rule.family, rule.badge])))
)

// The rule carrying `slug`, or `undefined` when no rule page matches.
export async function ruleBySlug(slug: string): Promise<DiscoveredRule | undefined> {
  return (await discoveredRules()).find(rule => rule.slug === slug)
}

// The rule carrying `slug`, throwing when no rule page matches so a bad
// slug fails the build at the component that passed it.
export async function ruleOrThrow(slug: string, consumer: string): Promise<DiscoveredRule> {
  return required(await ruleBySlug(slug), `${consumer} slug "${slug}" does not match a rule page`)
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
    if (page === undefined) continue
    const { family, slug } = page
    if (!isFamily(family)) continue
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
