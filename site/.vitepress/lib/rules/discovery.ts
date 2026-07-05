import fs   from 'node:fs'
import path from 'node:path'

import { contentPages, isContentPage, matterPages } from '../shared/content-page'
import { memoizeByPath }                            from '../shared/memoize-by-path'
import * as registries                              from '../shared/registries'
import { requireString }                            from '../shared/require-string'
import { ruleRoute }                                from '../shared/routes'

export interface DiscoveredRule {
  caption  : string
  category : registries.RuleCategory
  family   : registries.RuleFamily
  href     : string
  related  : readonly string[]
  slug     : string
}

export interface RuleDiscovery {
  rules      : DiscoveredRule[]
  strayPages : string[]
}

export const discoverRuleIndex = memoizeByPath(
  (rulesDirectory): ReadonlyMap<string, DiscoveredRule> =>
    new Map(discoverRules(rulesDirectory).rules.map(r => [r.slug, r]))
)

export const discoverRules = memoizeByPath((rulesDirectory): RuleDiscovery => {
  const families   = new Set<string>(registries.FAMILY_ORDER)
  const rules      : DiscoveredRule[] = []
  const strayPages : string[] = []
  for (const entry of fs.readdirSync(rulesDirectory, { withFileTypes: true })) {
    if (entry.isFile()) {
      if (isContentPage(entry.name)) strayPages.push(entry.name)
      continue
    }
    const directory = path.join(rulesDirectory, entry.name)
    if (!families.has(entry.name)) {
      strayPages.push(...contentPages(directory).map(f => `${entry.name}/${f}`))
      continue
    }
    const family = entry.name as registries.RuleFamily
    for (const { data: fm, slug } of matterPages(directory)) {
      const caption = requireString(
        fm.caption,
        `Rule "${slug}" has invalid or missing caption: ${JSON.stringify(fm.caption)}`
      )
      const relatedSlugs = Array.isArray(fm.related) ? fm.related as string[] : []
      rules.push({
        caption,
        category : registries.categoryOf(family),
        family,
        href     : ruleRoute(family, slug),
        related  : relatedSlugs,
        slug
      })
    }
  }
  rules.sort((a, b) => a.slug.localeCompare(b.slug))
  return { rules, strayPages }
})

export function discoverRuleSlugs(rulesDirectory: string): DiscoveredRule[] {
  return discoverRules(rulesDirectory).rules
}
