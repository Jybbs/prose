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

export const discoverRuleSlugs = memoizeByPath((rulesDirectory): DiscoveredRule[] => {
  const families = new Set<string>(FAMILY_ORDER)
  const out      : DiscoveredRule[] = []
  const stray    : string[] = []
  for (const entry of fs.readdirSync(rulesDirectory, { withFileTypes: true })) {
    if (entry.isFile()) {
      if (isContentPage(entry.name)) stray.push(entry.name)
      continue
    }
    const directory = path.join(rulesDirectory, entry.name)
    const pages     = fs.readdirSync(directory).filter(isContentPage)
    if (!families.has(entry.name)) {
      stray.push(...pages.map(f => `${entry.name}/${f}`))
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

  return out
})
