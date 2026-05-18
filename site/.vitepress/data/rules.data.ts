import { createContentLoader } from 'vitepress'

import type { RuleCategory } from '../lib/registries'
import type { Registry }     from '../lib/types'

export type { RuleCategory }

export interface DiscoveredRule {
  category : RuleCategory
  slug     : string
}

export interface RulesData {
  bySlug : Registry<DiscoveredRule>
  list   : readonly DiscoveredRule[]
}

declare const data: RulesData
export { data }

export default createContentLoader('rules/*.md', {
  transform(pages): RulesData {
    const parsed = pages
      .filter(p => !p.url.endsWith('/rules/'))
      .map(p => ({ frontmatter: p.frontmatter, slug: p.url.replace(/^\/rules\/|\/$/g, '') }))

    const slugs = new Set(parsed.map(p => p.slug))
    for (const { frontmatter, slug } of parsed) {
      const related = frontmatter.related
      if (Array.isArray(related)) {
        for (const ref of related) {
          if (!slugs.has(ref)) {
            throw new Error(`Rule "${slug}" lists invalid related slug "${ref}"`)
          }
        }
      }
    }

    const list = parsed
      .map(({ frontmatter, slug }): DiscoveredRule => {
        const category = frontmatter.category
        if (category !== 'auto-fix' && category !== 'lint') {
          throw new Error(`Rule "${slug}" has invalid or missing category: ${JSON.stringify(category)}`)
        }
        return { category, slug }
      })
      .sort((a, b) => a.slug.localeCompare(b.slug))

    const bySlug: Registry<DiscoveredRule> = Object.fromEntries(list.map(r => [r.slug, r]))
    return { bySlug, list }
  }
})
