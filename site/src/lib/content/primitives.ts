import { getCollection } from 'astro:content'

import { sectionLeaf }             from './sections'
import { PRIMITIVE_LAYERS }        from '../shared/registries'
import type { PrimitiveStability } from '../shared/registries'
import { primitiveRoute }          from '../shared/routes'

export type PrimitiveLayer = (typeof PRIMITIVE_LAYERS)[number]

export interface DiscoveredPrimitive {
  consumedBy : readonly string[]
  consumes   : readonly string[]
  href       : string
  layer      : PrimitiveLayer
  name       : string
  slug       : string
  stability  : PrimitiveStability
  summary    : string
  tagline    : string
}

export const LAYER_NUMERAL: Record<PrimitiveLayer, string> = {
  analysis      : 'III',
  base          : 'I',
  orchestration : 'II'
}

let cached: Promise<DiscoveredPrimitive[]> | null = null

// The primitive roster the composition surfaces render, read off the `docs`
// collection's primitive pages, sorted by slug and derived once per build. The
// graph edges are validated by the corpus-integrity pass, whereas the per-page
// fields are asserted here.
export function discoveredPrimitives(): Promise<DiscoveredPrimitive[]> {
  return (cached ??= derivePrimitives())
}

async function derivePrimitives(): Promise<DiscoveredPrimitive[]> {
  const out: DiscoveredPrimitive[] = []
  for (const entry of await getCollection('docs')) {
    const slug = sectionLeaf(entry.id, 'primitives')
    if (slug === undefined) continue
    const { consumedBy, consumes, layer, stability, summary, tagline } = entry.data
    if (layer === undefined || stability === undefined || summary === undefined || tagline === undefined) {
      throw new Error(`Primitive "${slug}" is missing layer, stability, summary, or tagline frontmatter`)
    }
    out.push({
      consumedBy : consumedBy ?? [],
      consumes   : consumes ?? [],
      href       : primitiveRoute(slug),
      layer,
      name       : entry.data.title,
      slug,
      stability,
      summary,
      tagline
    })
  }
  return out.sort((a, b) => a.slug.localeCompare(b.slug))
}
