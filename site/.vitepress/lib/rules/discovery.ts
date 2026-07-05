import fs   from 'node:fs'
import path from 'node:path'

import matter from 'gray-matter'

import { isContentPage } from '../shared/content-page'
import { memoizeByPath } from '../shared/memoize-by-path'
import { categoryOf, FAMILY_ORDER, type RuleCategory, type RuleFamily } from '../shared/registries'
import { requireString } from '../shared/require-string'

export interface DiscoveredRule {
  caption  : string
  category : RuleCategory
  family   : RuleFamily
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
  const families   = new Set<string>(FAMILY_ORDER)
  const rules      : DiscoveredRule[] = []
  const strayPages : string[] = []
  for (const entry of fs.readdirSync(rulesDirectory, { withFileTypes: true })) {
    if (entry.isFile()) {
      if (isContentPage(entry.name)) strayPages.push(entry.name)
      continue
    }
    const directory = path.join(rulesDirectory, entry.name)
    const pages     = fs.readdirSync(directory).filter(isContentPage)
    if (!families.has(entry.name)) {
      strayPages.push(...pages.map(f => `${entry.name}/${f}`))
      continue
    }
    const family = entry.name as RuleFamily
    for (const file of pages) {
      const slug    = path.basename(file, '.md')
      const fm      = matter.read(path.join(directory, file)).data
      const caption = requireString(
        fm.caption,
        `Rule "${slug}" has invalid or missing caption: ${JSON.stringify(fm.caption)}`
      )
      const relatedSlugs = Array.isArray(fm.related) ? fm.related as string[] : []
      rules.push({
        caption,
        category : categoryOf(family),
        family,
        href     : `/rules/${family}/${slug}`,
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
