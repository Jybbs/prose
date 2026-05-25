import { createContentLoader } from 'vitepress'

import { PRIMITIVES, assertCoversPrimitives, type PrimitiveSlug } from '../lib/shared/registries'

export interface DiscoveredPrimitive {
  display : string
  slug    : PrimitiveSlug
}

interface PrimitivesData {
  bySlug : Record<string, DiscoveredPrimitive>
  list   : readonly DiscoveredPrimitive[]
}

declare const data: PrimitivesData
export { data }

export default createContentLoader('primitives/*.md', {
  transform(pages): PrimitivesData {
    const collected = pages
      .filter(p => !p.url.endsWith('/primitives/'))
      .map(p => p.url.replace(/^\/primitives\/|\/$/g, ''))
    assertCoversPrimitives(collected, 'primitive pages')
    const list = Object.entries(PRIMITIVES)
      .map(([slug, display]): DiscoveredPrimitive => ({ display, slug: slug as PrimitiveSlug }))
      .sort((a, b) => a.slug.localeCompare(b.slug))
    const bySlug: Record<string, DiscoveredPrimitive> = Object.fromEntries(list.map(p => [p.slug, p]))
    return { bySlug, list }
  }
})
