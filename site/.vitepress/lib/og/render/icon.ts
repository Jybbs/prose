import type { BrandAssets } from './assets'
import * as parts           from './parts'

const GLYPH_DROP = 4
const GLYPH_SIZE = 140
const INSET      = (parts.ICON_SIZE - GLYPH_SIZE) / 2

// Draws the pen-nib P on the card shell as a square icon, centered horizontally and
// `GLYPH_DROP` pixels below center vertically.
export function iconSvg(brand: BrandAssets): Promise<string> {
  const glyph = parts.el('img', {
    height : GLYPH_SIZE,
    src    : brand.glyph,
    style  : { left: INSET, position: 'absolute', top: INSET + GLYPH_DROP },
    width  : GLYPH_SIZE
  })
  return parts.toSvg(parts.cardShell(glyph), brand.fonts, parts.ICON_SIZE, parts.ICON_SIZE)
}
