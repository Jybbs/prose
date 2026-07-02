import * as cacache from 'cacache'
import { hash }     from 'ohash'

import resolveSource  from '../tokens/resolve.ts?raw'
import assetsSource   from './assets.ts?raw'
import colorsSource   from './colors.ts?raw'
import landingSource  from './landing.ts?raw'
import partsSource    from './parts.ts?raw'
import templateSource from './template.ts?raw'

import type { BrandAssets } from './assets'
import type { OgPage }      from './pages'

// The render sources folded into every cache key, so an edit to any of them
// re-renders every card.
const TEMPLATE_DIGEST = hash([
  assetsSource, colorsSource, landingSource, partsSource, resolveSource, templateSource
])

export function cardKeyer(version: string, brand: BrandAssets): (card: OgPage | 'landing') => string {
  const base = { brand: hash(brand), template: TEMPLATE_DIGEST, version }
  return card => hash({ base, card })
}

export async function readCard(cacheDir: string, key: string): Promise<Buffer | null> {
  try {
    return (await cacache.get(cacheDir, key)).data
  }
  catch {
    // a miss and a failed integrity check both fall through to a fresh render
    return null
  }
}

export async function writeCard(cacheDir: string, key: string, png: Buffer): Promise<void> {
  try {
    await cacache.put(cacheDir, key, png)
  }
  catch {
    // a failed write still leaves the rendered card in the response
  }
}

export async function pruneCards(cacheDir: string, live: Iterable<string>): Promise<void> {
  try {
    const keep  = new Set(live)
    const index = await cacache.ls(cacheDir)
    const stale = Object.keys(index).filter(key => !keep.has(key))
    if (stale.length === 0) return
    await Promise.all(stale.map(key => cacache.rm.entry(cacheDir, key)))
    await cacache.verify(cacheDir)
  }
  catch {
    // prune is best-effort housekeeping
  }
}
