import fs                from 'node:fs'
import path              from 'node:path'
import { fileURLToPath } from 'node:url'

import * as cacache from 'cacache'
import { hash }     from 'ohash'

import type { BrandAssets } from './assets'
import type { OgPage }      from './pages'

const OG_DIR = path.dirname(fileURLToPath(import.meta.url))

// render sources folded into every cache key, so a change here re-renders all cards
const TEMPLATE_FILES: readonly string[] = [
  'assets.ts', 'landing.ts', 'parts.ts', 'template.ts', '../shared/registries.ts'
]

export type CardInput = OgPage | 'landing'

export function cardKeyer(version: string, brand: BrandAssets): (card: CardInput) => string {
  const base = { brand: hash(brand), template: templateDigest(), version }
  return card => hash({ base, card })
}

export async function readCard(cacheDir: string, key: string): Promise<Buffer | null> {
  try {
    return (await cacache.get(cacheDir, key)).data
  }
  catch {
    // a miss or a failed integrity check both fall through to a fresh render
    return null
  }
}

export async function writeCard(cacheDir: string, key: string, png: Buffer): Promise<void> {
  try {
    await cacache.put(cacheDir, key, png)
  }
  catch {
    // a failed write still leaves the rendered card in dist
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

function templateDigest(): string {
  return hash(TEMPLATE_FILES.map(file => fs.readFileSync(path.join(OG_DIR, file))))
}
