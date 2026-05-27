import { Resvg }              from '@resvg/resvg-js'
import satori, { type Font }  from 'satori'

import type { BrandAssets }                                  from './assets'
import type { OgPage }                                       from './pages'
import { CARD_HEIGHT, CARD_WIDTH, buildCard, type JsxNode }  from './template'

export async function renderPage(page: OgPage, brand: BrandAssets, version: string): Promise<Buffer> {
  return renderCard(buildCard(page, version, brand.wordmark, brand.glyph), brand.fonts)
}

async function renderCard(node: JsxNode, fonts: Font[]): Promise<Buffer> {
  const svg = await satori(node, { fonts, height: CARD_HEIGHT, width: CARD_WIDTH })
  return new Resvg(svg).render().asPng()
}
