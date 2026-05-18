import { createContentLoader } from 'vitepress'

import { PRIMITIVES, type PrimitiveSlug } from '../lib/primitives'

export interface DiscoveredPrimitive {
  display : string
  slug    : PrimitiveSlug
}

declare const data: DiscoveredPrimitive[]
export { data }

export default createContentLoader('primitives/*.md', {
  transform(pages): DiscoveredPrimitive[] {
    const known     = new Set(Object.keys(PRIMITIVES))
    const found     = new Set<string>()
    const collected = pages
      .filter(p => !p.url.endsWith('/primitives/'))
      .map(p => p.url.replace(/^\/primitives\/|\/$/g, ''))
    for (const slug of collected) {
      if (!known.has(slug)) {
        throw new Error(`primitive page "${slug}" is not in PRIMITIVES registry`)
      }
      found.add(slug)
    }
    const missing = [...known].filter(s => !found.has(s))
    if (missing.length > 0) {
      throw new Error(`PRIMITIVES registry has no matching page: [${missing.join(', ')}]`)
    }
    return Object.entries(PRIMITIVES)
      .map(([slug, display]) => ({ display, slug: slug as PrimitiveSlug }))
      .sort((a, b) => a.slug.localeCompare(b.slug))
  }
})
