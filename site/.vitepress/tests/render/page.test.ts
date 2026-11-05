import { formatRgb } from 'culori'

import * as palette from '../../lib/shared/palette'
import { huesIn }   from './page'

const computed = (hex: string): string => formatRgb(hex)!

// Lists one scheme's shades of the VitePress text colors or, where `ofText` is
// false, its shades of the palette's hues.
const shades = (ofText: boolean, scheme: palette.Scheme): string[] =>
  Object.entries(palette.SCHEME_SHADES[scheme])
    .filter(([name]) => name.startsWith('text-') === ofText)
    .map(([, hex]) => hex)

describe('huesIn', () => {
  it.each(palette.SCHEMES)('finds no hue in the %s VitePress colors or the shades of their text', scheme => {
    const vitepress = Object.values(palette.VITEPRESS).map(colors => colors[scheme])

    expect(huesIn([...vitepress, ...shades(true, scheme)].map(computed).join(' '))).toStrictEqual([])
  })

  it.each(palette.SCHEMES)('finds a hue in every %s role and every shade of a palette hue', scheme => {
    const hues = [...Object.values(palette.SCHEME_ROLES[scheme]), ...shades(false, scheme)].map(computed)

    expect(huesIn(hues.join(' '))).toStrictEqual(hues)
  })

  it.each(['color(srgb 0.38 0.35 0.57 / 0.4)', 'oklch(0.5 0.1 270)', 'rgba(97, 90, 145, 0.5)'])('finds the hue in a computed %s', color => {
    expect(huesIn(color)).toStrictEqual([color])
  })

  it('leaves the neutral color out of a value holding two', () => {
    expect(huesIn('rgb(97, 90, 145) rgb(152, 152, 159)')).toStrictEqual(['rgb(97, 90, 145)'])
  })

  it('finds nothing in a value holding no color', () => {
    expect(huesIn('auto')).toStrictEqual([])
  })
})
