import type { BrandAssets }   from './assets'
import type { OgPage }        from './pages'
import { rasterize }          from './parts'
import { buildCard }          from './template'

export async function renderPage(
  page    : OgPage,
  brand   : BrandAssets,
  version : string
): Promise<Buffer> {
  return rasterize(buildCard(page, version, brand.wordmark, brand.glyph), brand.fonts)
}
