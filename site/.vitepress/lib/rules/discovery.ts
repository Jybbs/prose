import fs   from 'node:fs'
import path from 'node:path'

import matter from 'gray-matter'

import { FAMILY_ORDER, type RuleCategory, type RuleFamily } from '../shared/registries'

export interface DiscoveredRule {
  caption  : string
  category : RuleCategory
  family   : RuleFamily
  href     : string
  related  : readonly string[]
  slug     : string
}

const cache = new Map<string, DiscoveredRule[]>()

export function discoverRuleSlugs(rulesDirectory: string): DiscoveredRule[] {
  const cached = cache.get(rulesDirectory)
  if (cached !== undefined) return cached

  const stray = fs.readdirSync(rulesDirectory).filter(f => f.endsWith('.md') && f !== 'index.md')
  if (stray.length > 0) {
    throw new Error(`Rule pages must live in a family directory, found stray: ${stray.join(', ')}`)
  }

  const out    : DiscoveredRule[] = []
  const related: Array<{ refs: readonly string[]; slug: string }> = []
  for (const family of FAMILY_ORDER) {
    const familyDirectory = path.join(rulesDirectory, family)
    if (!fs.existsSync(familyDirectory)) continue
    for (const file of fs.readdirSync(familyDirectory).sort()) {
      if (!file.endsWith('.md') || file === 'index.md') continue
      const slug    = file.slice(0, -'.md'.length)
      const body    = fs.readFileSync(path.join(familyDirectory, file), 'utf8')
      const fm      = matter(body).data
      const caption = fm.caption
      if (typeof caption !== 'string' || caption.trim() === '') {
        throw new Error(`Rule "${slug}" has invalid or missing caption: ${JSON.stringify(caption)}`)
      }
      const relatedSlugs = Array.isArray(fm.related) ? fm.related as string[] : []
      out.push({
        caption,
        category : family === 'lint' ? 'lint' : 'auto-fix',
        family,
        href     : `/rules/${family}/${slug}`,
        related  : relatedSlugs,
        slug
      })
      if (relatedSlugs.length > 0) related.push({ refs: relatedSlugs, slug })
    }
  }

  out.sort((a, b) => a.slug.localeCompare(b.slug))

  const known = new Set(out.map(r => r.slug))
  for (const { refs, slug } of related) {
    for (const ref of refs) {
      if (!known.has(ref)) throw new Error(`Rule "${slug}" lists invalid related slug "${ref}"`)
    }
  }

  cache.set(rulesDirectory, out)
  return out
}
