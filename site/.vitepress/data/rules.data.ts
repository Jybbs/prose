import { createContentLoader } from 'vitepress'

import type { RuleCategory } from '../lib/categories'

export type { RuleCategory }

export interface DiscoveredRule {
  category : RuleCategory
  slug     : string
}

declare const data: DiscoveredRule[]
export { data }

function assertCategory(value: unknown, url: string): RuleCategory {
  if (value === 'auto-fix' || value === 'lint') return value
  throw new Error(`Rule ${url} has invalid category: ${JSON.stringify(value)}`)
}

export default createContentLoader('rules/*.md', {
  transform(pages): DiscoveredRule[] {
    const all   = pages.filter(p => !p.url.endsWith('/rules/'))
    const slugs = new Set(all.map(p => p.url.replace(/^\/rules\/|\/$/g, '')))

    return all
      .map(p => {
        const slug    = p.url.replace(/^\/rules\/|\/$/g, '')
        const related = p.frontmatter.related as string[] | undefined
        related?.forEach(r => {
          if (!slugs.has(r)) {
            throw new Error(`Rule "${slug}" has invalid related slug: "${r}"`)
          }
        })
        return { slug, category: assertCategory(p.frontmatter.category, p.url) }
      })
      .sort((a, b) => a.slug.localeCompare(b.slug))
  }
})
