import { createContentLoader } from 'vitepress'

import { PRIMITIVES, type PrimitiveSlug } from '../lib/shared/registries'
import type { Registry }                  from '../lib/shared/types'

export interface DiscoveredPrimitive {
  display : string
  slug    : PrimitiveSlug
}

export interface PrimitivesData {
  bySlug : Registry<DiscoveredPrimitive>
  list   : readonly DiscoveredPrimitive[]
}

declare const data: PrimitivesData
export { data }

export default createContentLoader('primitives/*.md', {
  transform(pages): PrimitivesData {
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
    const list = Object.entries(PRIMITIVES)
      .map(([slug, display]): DiscoveredPrimitive => ({ display, slug: slug as PrimitiveSlug }))
      .sort((a, b) => a.slug.localeCompare(b.slug))
    const bySlug: Registry<DiscoveredPrimitive> = Object.fromEntries(list.map(p => [p.slug, p]))
    return { bySlug, list }
  }
})
