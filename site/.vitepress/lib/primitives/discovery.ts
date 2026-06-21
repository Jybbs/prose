import fs   from 'node:fs'
import path from 'node:path'

import matter from 'gray-matter'

import { markdownH1 }                              from '../markdown/h1'
import { type PrimitiveLayer, type PrimitiveSlug } from '../shared/registries'

const LAYERS: readonly PrimitiveLayer[] = ['analysis', 'base', 'orchestration']

export interface DiscoveredPrimitive {
  consumedBy : readonly string[]
  consumes   : readonly PrimitiveSlug[]
  layer      : PrimitiveLayer
  name       : string
  slug       : PrimitiveSlug
  stability  : 'internal' | 'public'
  summary    : string
  tagline    : string
}

const cache = new Map<string, DiscoveredPrimitive[]>()

function stringList(value: unknown, slug: string, field: string): string[] {
  if (!Array.isArray(value) || value.some(v => typeof v !== 'string')) {
    throw new Error(`Primitive "${slug}" has invalid or missing ${field}`)
  }
  return value as string[]
}

export function discoverPrimitives(primitivesDir: string): DiscoveredPrimitive[] {
  const cached = cache.get(primitivesDir)
  if (cached !== undefined) return cached

  const out: DiscoveredPrimitive[] = []
  for (const file of fs.readdirSync(primitivesDir).sort()) {
    if (!file.endsWith('.md') || file === 'index.md') continue
    const slug = path.basename(file, '.md') as PrimitiveSlug
    const fm   = matter.read(path.join(primitivesDir, file))

    const { layer, stability, summary, tagline } = fm.data
    if (stability !== 'public' && stability !== 'internal') {
      throw new Error(
        `Primitive "${slug}" has invalid or missing stability: ${JSON.stringify(stability)}`
      )
    }
    if (!LAYERS.includes(layer as PrimitiveLayer)) {
      throw new Error(`Primitive "${slug}" has invalid or missing layer: ${JSON.stringify(layer)}`)
    }
    if (typeof summary !== 'string' || summary === '') {
      throw new Error(`Primitive "${slug}" has invalid or missing summary`)
    }
    if (typeof tagline !== 'string' || tagline === '') {
      throw new Error(`Primitive "${slug}" has invalid or missing tagline`)
    }

    const consumes   = stringList(fm.data.consumes, slug, 'consumes') as PrimitiveSlug[]
    const consumedBy = stringList(fm.data.consumedBy, slug, 'consumedBy')

    const name = markdownH1(fm.content)
    if (!name) {
      throw new Error(`Primitive "${slug}" has no H1 heading`)
    }

    out.push({ consumedBy, consumes, layer, name, slug, stability, summary, tagline })
  }

  const slugs = new Set(out.map(p => p.slug))
  for (const p of out) {
    for (const dep of p.consumes) {
      if (!slugs.has(dep)) throw new Error(`Primitive "${p.slug}" consumes unknown primitive "${dep}"`)
    }
  }

  cache.set(primitivesDir, out)
  return out
}
