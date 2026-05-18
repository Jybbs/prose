import fs     from 'node:fs'
import path   from 'node:path'

import matter from 'gray-matter'

import type { RuleCategory } from '../shared/registries'

export interface DiscoveredRule {
  category : RuleCategory
  slug     : string
}

const cache = new Map<string, DiscoveredRule[]>()

export function discoverRuleSlugs(rulesDirectory: string): DiscoveredRule[] {
  const cached = cache.get(rulesDirectory)
  if (cached !== undefined) return cached

  const out    : DiscoveredRule[] = []
  const related: Array<{ refs: readonly string[]; slug: string }> = []
  for (const file of fs.readdirSync(rulesDirectory).sort()) {
    if (!file.endsWith('.md') || file === 'index.md') continue
    const slug     = file.slice(0, -'.md'.length)
    const body     = fs.readFileSync(path.join(rulesDirectory, file), 'utf8')
    const fm       = matter(body).data
    const category = fm.category
    if (category !== 'auto-fix' && category !== 'lint') {
      throw new Error(`Rule "${slug}" has invalid or missing category: ${JSON.stringify(category)}`)
    }
    out.push({ category, slug })
    if (Array.isArray(fm.related)) related.push({ refs: fm.related, slug })
  }

  const known = new Set(out.map(r => r.slug))
  for (const { refs, slug } of related) {
    for (const ref of refs) {
      if (!known.has(ref)) throw new Error(`Rule "${slug}" lists invalid related slug "${ref}"`)
    }
  }

  cache.set(rulesDirectory, out)
  return out
}

export function splitByCategory(rules: readonly DiscoveredRule[]): { autoFix: string[]; lint: string[] } {
  const autoFix: string[] = []
  const lint   : string[] = []
  for (const r of rules) {
    if (r.category === 'lint') lint.push(r.slug)
    else                       autoFix.push(r.slug)
  }
  return { autoFix, lint }
}
