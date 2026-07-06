import fs   from 'node:fs'
import path from 'node:path'

import * as cacache from 'cacache'
import { hash }     from 'ohash'

import type { BrandAssets } from './assets'
import type { OgPage }      from '../pages'

const OG_DIR = import.meta.dirname

const SHARED_SOURCES: readonly string[] = ['../../shared/palette.ts', '../../shared/registries.ts']

const TEMPLATE_DIGEST = hash(
  [...fs.readdirSync(OG_DIR).filter(file => !file.startsWith('.')).sort(), ...SHARED_SOURCES]
    .map(file => fs.readFileSync(path.join(OG_DIR, file), 'utf8'))
)

type CardInput = OgPage | 'landing'

export function cardKeyer(version: string, brand: BrandAssets): (card: CardInput) => string {
  const base = { brand: hash(brand), template: TEMPLATE_DIGEST, version }
  return card => hash({ base, card })
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
    // Prune is best-effort housekeeping
  }
}

export async function readCard(cacheDir: string, key: string): Promise<Buffer | null> {
  try {
    return (await cacache.get(cacheDir, key)).data
  }
  catch {
    // A miss or a failed integrity check both fall through to a fresh render
    return null
  }
}

export async function writeCard(cacheDir: string, key: string, png: Buffer): Promise<void> {
  try {
    await cacache.put(cacheDir, key, png)
  }
  catch {
    // A failed write still leaves the rendered card in dist
  }
}
