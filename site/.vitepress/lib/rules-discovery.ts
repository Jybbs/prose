import fs     from 'node:fs'
import path   from 'node:path'

import matter from 'gray-matter'

import type { RuleCategory } from './registries'

export interface DiscoveredRuleSlug {
  category : RuleCategory
  slug     : string
}

export function discoverRuleSlugs(rulesDirectory: string): DiscoveredRuleSlug[] {
  const out: DiscoveredRuleSlug[] = []
  for (const file of fs.readdirSync(rulesDirectory).sort()) {
    if (!file.endsWith('.md') || file === 'index.md') continue
    const slug     = file.slice(0, -'.md'.length)
    const body     = fs.readFileSync(path.join(rulesDirectory, file), 'utf8')
    const category = matter(body).data.category
    if (category !== 'auto-fix' && category !== 'lint') {
      throw new Error(`Rule "${slug}" has invalid or missing category: ${JSON.stringify(category)}`)
    }
    out.push({ category, slug })
  }
  return out
}

export function splitByCategory(rules: readonly DiscoveredRuleSlug[]): { autoFix: string[]; lint: string[] } {
  const autoFix: string[] = []
  const lint   : string[] = []
  for (const r of rules) {
    if (r.category === 'lint') lint.push(r.slug)
    else                       autoFix.push(r.slug)
  }
  return { autoFix, lint }
}
