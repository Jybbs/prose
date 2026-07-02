import { fileURLToPath } from 'node:url'

import { root }          from 'astro:config/server'
import { getCollection } from 'astro:content'
import sharp             from 'sharp'

import { type BrandAssets, loadBrandAssets }          from './assets'
import { cardKeyer, pruneCards, readCard, writeCard } from './cache'
import { landingSvg }                                 from './landing'
import { type OgCard, enumerateCards }                from './pages'
import { pageSvg }                                    from './template'

const CACHE_DIR = fileURLToPath(new URL('../.cache/og/', root))

interface Renderer {
  brand   : BrandAssets
  cards   : ReadonlyMap<string, OgCard & { key: string }>
  version : string
}

let renderer: Promise<Renderer> | undefined

export async function cardResponse(id: string): Promise<Response> {
  return new Response(new Uint8Array(await renderCard(id)), {
    headers: { 'Content-Type': 'image/png' }
  })
}

export async function pageCardIds(): Promise<string[]> {
  const { cards } = await (renderer ??= init())
  return [...cards.values()].filter(card => card.page !== 'landing').map(card => card.id)
}

async function renderCard(id: string): Promise<Buffer> {
  const { brand, cards, version } = await (renderer ??= init())
  const card = cards.get(id)
  if (card === undefined) throw new Error(`no OG card enumerated for "${id}"`)
  const cached = await readCard(CACHE_DIR, card.key)
  if (cached !== null) return cached
  const svg = card.page === 'landing'
    ? await landingSvg(brand, version)
    : await pageSvg(card.page, brand, version)
  const png = await sharp(Buffer.from(svg)).png().toBuffer()
  await writeCard(CACHE_DIR, card.key, png)
  return png
}

async function init(): Promise<Renderer> {
  const [[release], enumerated] = await Promise.all([getCollection('release'), enumerateCards()])
  const brand   = loadBrandAssets()
  const version = release.data.version
  const keyOf   = cardKeyer(version, brand)
  const cards   = new Map(enumerated.map(card => [card.id, { ...card, key: keyOf(card.page) }]))
  await pruneCards(CACHE_DIR, [...cards.values()].map(card => card.key))
  return { brand, cards, version }
}
