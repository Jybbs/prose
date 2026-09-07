import fs   from 'node:fs'
import path from 'node:path'

import * as contentPage  from '../shared/content-page'
import * as paths        from '../shared/paths'
import * as registries   from '../shared/registries'
import { requireString } from '../shared/require-string'
import { ruleRoute }     from '../shared/routes'

export interface DiscoveredRule {
  caption  : string
  category : registries.RuleCategory
  family   : registries.RuleFamily
  href     : string
  lints    : boolean
  related  : readonly string[]
  slug     : string
}

export interface RuleDiscovery {
  rules      : DiscoveredRule[]
  strayPages : string[]
}

const crate = paths.crateDirFrom(paths.repoRoot(import.meta.url))

// A rule emits lints where its module overrides `Rule::lint`, read off the
// crate source the way `crate/tests/site.rs` reads it.
function moduleEmitsLints(slug: string): boolean {
  const directory = path.join(crate, 'src', 'rules', slug.replaceAll('-', '_'))
  if (!fs.existsSync(directory)) return false
  return (fs.readdirSync(directory, { recursive: true }) as string[])
    .filter(name => name.endsWith('.rs'))
    .some(name => fs.readFileSync(path.join(directory, name), 'utf8').includes('fn lint(&self'))
}

export function discoverRuleIndex(rulesDirectory: string): ReadonlyMap<string, DiscoveredRule> {
  return new Map(discoverRules(rulesDirectory).rules.map(r => [r.slug, r]))
}

export function discoverRules(rulesDirectory: string): RuleDiscovery {
  const families   = new Set<string>(registries.FAMILY_ORDER)
  const rules      : DiscoveredRule[] = []
  const strayPages : string[] = []
  for (const entry of fs.readdirSync(rulesDirectory, { withFileTypes: true })) {
    if (entry.isFile()) {
      if (contentPage.isContentPage(entry.name)) strayPages.push(entry.name)
      continue
    }
    const directory = path.join(rulesDirectory, entry.name)
    if (!families.has(entry.name)) {
      strayPages.push(...contentPage.contentPages(directory).map(f => `${entry.name}/${f}`))
      continue
    }
    const family = entry.name as registries.RuleFamily
    for (const { data: fm, slug } of contentPage.matterPages(directory)) {
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
        lints    : registries.categoryOf(family) === 'lint' || moduleEmitsLints(slug),
        related  : relatedSlugs,
        slug
      })
    }
  }
  rules.sort((a, b) => a.slug.localeCompare(b.slug))
  return { rules, strayPages }
}

export function discoverRuleSlugs(rulesDirectory: string): DiscoveredRule[] {
  return discoverRules(rulesDirectory).rules
}
