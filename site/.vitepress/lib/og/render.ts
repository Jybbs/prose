import { Resvg }                              from '@resvg/resvg-js'
import satori                                 from 'satori'

import type { BrandAssets }                   from './assets'
import type { OgPage }                        from './pages'
import { CARD_HEIGHT, CARD_WIDTH, buildCard } from './template'

export async function renderPage(page: OgPage, brand: BrandAssets, version: string): Promise<Buffer> {
  const node = buildCard(page, version, brand.wordmark, brand.glyph)
  const svg  = await satori(node, { fonts: brand.fonts, height: CARD_HEIGHT, width: CARD_WIDTH })
  return new Resvg(svg).render().asPng()
}
