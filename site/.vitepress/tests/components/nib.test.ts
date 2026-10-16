// @vitest-environment happy-dom
import { Resvg } from '@resvg/resvg-js'
import { mount } from '@vue/test-utils'

import { familyEmblem, MOTIFS } from '../../lib/shared/emblems'
import * as nib                 from '../../lib/shared/nib'
import NibBadge                 from '../../theme/components/base/NibBadge.vue'
import NibSprite                from '../../theme/components/base/NibSprite.vue'

const PAD    = 3
const RASTER = 480

const sprite  = mount(NibSprite)
const SYMBOLS = MOTIFS.flatMap(motif =>
  (['large', 'small'] as const).map(size => ({ id: nib.nibId(motif, size), motif })))

// Measures the ink a symbol lays down on the 24-unit grid, the box around
// every pixel at half alpha or more, without the jitter a large symbol's
// filter adds.
function inkBox(
  { id, motif } : (typeof SYMBOLS)[number]
): { cx: number, cy: number, height: number, width: number } {
  const body   = sprite.get(`#${id}`).element.innerHTML.replace(/ filter="[^"]*"/, '')
  const paths  = sprite.get(`#${nib.nibId(motif, 'paths')}`).element.outerHTML
  const span   = 24 + 2 * PAD
  const svg    = `<svg xmlns="http://www.w3.org/2000/svg" color="#000" `
               + `width="${RASTER}" height="${RASTER}" viewBox="${-PAD} ${-PAD} ${span} ${span}">`
               + `<defs>${paths}</defs>${body}</svg>`
  const pixels = new Resvg(svg, { font: { loadSystemFonts: false } }).render().pixels
  const unit   = span / RASTER
  let [x0, y0, x1, y1] = [Infinity, Infinity, -Infinity, -Infinity]
  for (let y = 0; y < RASTER; y++) {
    for (let x = 0; x < RASTER; x++) {
      if (pixels[(y * RASTER + x) * 4 + 3] < 128) continue
      x0 = Math.min(x0, x)
      y0 = Math.min(y0, y)
      x1 = Math.max(x1, x + 1)
      y1 = Math.max(y1, y + 1)
    }
  }
  const at = (px: number): number => px * unit - PAD
  return {
    cx     : (at(x0) + at(x1)) / 2,
    cy     : (at(y0) + at(y1)) / 2,
    height : at(y1) - at(y0),
    width  : at(x1) - at(x0)
  }
}

describe('NibSprite', () => {
  it.each(SYMBOLS)('declares $id on the 24-unit grid', ({ id }) => {
    expect(sprite.get(`#${id}`).attributes()).toMatchObject({ overflow: 'visible', viewBox: '0 0 24 24' })
  })

  it.each(MOTIFS)('roughens the large $label through its own filter', motif => {
    const jitter = nib.nibId(motif, 'jitter')
    expect(sprite.get(`#${nib.nibId(motif, 'large')} > g`).attributes('filter')).toBe(`url(#${jitter})`)
    expect(sprite.get(`#${jitter} feTurbulence`).attributes('type')).toBe('fractalNoise')
  })

  it.each(MOTIFS)('draws the small $label from its small drawing or from the nib copies', motif => {
    expect(sprite.findAll(`#${nib.nibId(motif, 'small')} use`))
      .toHaveLength(motif.small ? 0 : nib.NIB.small.length)
  })

  it.each(SYMBOLS)('sets $id 18 units tall, no wider than 22, centered on the grid', symbol => {
    const box = inkBox(symbol)
    expect(box.height).toBeGreaterThan(17.6)
    expect(box.height).toBeLessThan(18.2)
    expect(box.width).toBeLessThan(22.5)
    expect(box.cx).toBeCloseTo(12, 1)
    expect(box.cy).toBeCloseTo(12, 1)
  })
})

describe('NibBadge', () => {
  const emblem = familyEmblem('docs')

  it.each([
    [14,   undefined],
    [96,   undefined],
    [15.2, 'Docs']
  ])('renders the markup nibBadgeHtml writes at %s px, labeled %s', (size, label) => {
    expect(mount(NibBadge, { props: { emblem, label, size } }).html({ raw: true }))
      .toBe(nib.nibBadgeHtml(emblem, size, label))
  })
})
