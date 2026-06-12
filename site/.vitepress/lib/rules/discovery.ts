import fs   from 'node:fs'
import path from 'node:path'

import matter from 'gray-matter'

import { categoryOf, FAMILY_ORDER, type RuleCategory, type RuleFamily } from '../shared/registries'

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

  const families = new Set<string>(FAMILY_ORDER)
  const out      : DiscoveredRule[] = []
  const stray    : string[] = []
  for (const entry of fs.readdirSync(rulesDirectory, { withFileTypes: true })) {
    if (entry.isFile()) {
      if (entry.name.endsWith('.md') && entry.name !== 'index.md') stray.push(entry.name)
      continue
    }
    const directory = path.join(rulesDirectory, entry.name)
    const pages     = fs.readdirSync(directory).filter(f => f.endsWith('.md') && f !== 'index.md')
    if (!families.has(entry.name)) {
      stray.push(...pages.map(f => `${entry.name}/${f}`))
      continue
    }
    const family = entry.name as RuleFamily
    for (const file of pages) {
      const slug    = file.slice(0, -'.md'.length)
      const body    = fs.readFileSync(path.join(directory, file), 'utf8')
      const fm      = matter(body).data
      const caption = fm.caption
      if (typeof caption !== 'string' || caption.trim() === '') {
        throw new Error(`Rule "${slug}" has invalid or missing caption: ${JSON.stringify(caption)}`)
      }
      const relatedSlugs = Array.isArray(fm.related) ? fm.related as string[] : []
      out.push({
        caption,
        category : categoryOf(family),
        family,
        href     : `/rules/${family}/${slug}`,
        related  : relatedSlugs,
        slug
      })
    }
  }
  if (stray.length > 0) {
    throw new Error(`Rule pages must live in a family directory, found stray: ${stray.join(', ')}`)
  }

  out.sort((a, b) => a.slug.localeCompare(b.slug))

  const known = new Set<string>()
  for (const { slug } of out) {
    if (known.has(slug)) {
      throw new Error(`Rule "${slug}" has pages in more than one family directory`)
    }
    known.add(slug)
  }
  for (const { related, slug } of out) {
    for (const ref of related) {
      if (!known.has(ref)) throw new Error(`Rule "${slug}" lists invalid related slug "${ref}"`)
    }
  }

  cache.set(rulesDirectory, out)
  return out
}
