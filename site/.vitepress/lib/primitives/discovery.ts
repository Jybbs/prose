import fs   from 'node:fs'
import path from 'node:path'

import matter from 'gray-matter'

import { type PrimitiveSlug } from '../shared/registries'

export interface DiscoveredPrimitive {
  name      : string
  slug      : PrimitiveSlug
  stability : 'internal' | 'public'
}

const cache = new Map<string, DiscoveredPrimitive[]>()

export function discoverPrimitives(primitivesDir: string): DiscoveredPrimitive[] {
  const cached = cache.get(primitivesDir)
  if (cached !== undefined) return cached

  const out: DiscoveredPrimitive[] = []
  for (const file of fs.readdirSync(primitivesDir).sort()) {
    if (!file.endsWith('.md') || file === 'index.md') continue
    const slug = file.slice(0, -'.md'.length) as PrimitiveSlug
    const body = fs.readFileSync(path.join(primitivesDir, file), 'utf8')
    const fm   = matter(body)

    const stability = fm.data.stability
    if (stability !== 'public' && stability !== 'internal') {
      throw new Error(
        `Primitive "${slug}" has invalid or missing stability: ${JSON.stringify(stability)}`
      )
    }

    const name = fm.content.match(/^#\s+(.+?)\s*$/m)?.[1]
    if (!name) {
      throw new Error(`Primitive "${slug}" has no H1 heading`)
    }

    out.push({ name, slug, stability })
  }

  cache.set(primitivesDir, out)
  return out
}
