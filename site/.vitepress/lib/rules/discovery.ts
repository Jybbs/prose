import fs     from 'node:fs'
import path   from 'node:path'

import matter from 'gray-matter'

import { FAMILY_META, type RuleCategory, type RuleFamily } from '../shared/registries'

export interface DiscoveredRule {
  caption  : string
  category : RuleCategory
  family   : RuleFamily
  related  : readonly string[]
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
    const family = fm.family
    if (typeof family !== 'string' || !(family in FAMILY_META)) {
      throw new Error(`Rule "${slug}" has invalid or missing family: ${JSON.stringify(family)}`)
    }
    if ((category === 'lint') !== (family === 'lint')) {
      throw new Error(`Rule "${slug}" mismatched category/family (${category}/${family}), because the lint family pairs exclusively with the lint category`)
    }
    const caption = fm.caption
    if (typeof caption !== 'string' || caption.trim() === '') {
      throw new Error(`Rule "${slug}" has invalid or missing caption: ${JSON.stringify(caption)}`)
    }
    const relatedSlugs = Array.isArray(fm.related) ? fm.related as string[] : []
    out.push({ caption, category, family: family as RuleFamily, related: relatedSlugs, slug })
    if (relatedSlugs.length > 0) related.push({ refs: relatedSlugs, slug })
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
