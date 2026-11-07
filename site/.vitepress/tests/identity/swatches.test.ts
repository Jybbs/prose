import { wcagContrast } from 'culori'

import * as swatches from '../../lib/identity/swatches'
import * as palette  from '../../lib/shared/palette'

const PAGE = { dark: '#1b1b1f', light: '#ffffff' }

describe('contrastOn', () => {
  it.each([
    ['#000000', 'text'],
    ['#8a80cb', 'mark'],
    ['#f6f8fa', 'fails']
  ] as const)('grades %s on white as %s', (color, level) => {
    expect(swatches.contrastOn(color, '#ffffff')).toStrictEqual({ level, ratio: wcagContrast(color, '#ffffff') })
  })
})

describe('oklchText', () => {
  it('writes the lightness and the chroma to three places and the hue to a tenth of a degree', () => {
    expect(swatches.oklchText('#8a80cb')).toMatch(/^0\.\d{3} 0\.\d{3} \d+\.\d°$/)
  })
})

describe('calloutMarks', () => {
  it('reads the property each kind’s mark reads, a kind with no rule of its own taking the shared one', () => {
    const sheet = '.vp-doc .custom-block:is(.tip, .info) { --callout-mark: var(--vp-c-text-2); }\n.vp-doc .custom-block.tip { --callout-mark: var(--color-link); }'
    expect(swatches.calloutMarks(sheet)).toStrictEqual(new Map([['tip', '--color-link'], ['info', '--vp-c-text-2']]))
  })
})

describe('calloutStates', () => {
  it('names the callout kind whose mark each hue’s shade inks', () => {
    const marks = new Map([['warning', '--prose-shade-eureka-page-45'], ['tip', '--color-link']])
    expect(swatches.calloutStates(marks)).toStrictEqual(new Map([['eureka', 'warning']]))
  })
})

describe('scanUses', () => {
  const uses = swatches.scanUses(
    [{ file: 'a.css', text: '.x { color: var(--color-ube-deep); }\n.dark .y { border-color: var(--color-ube); }\n.z { --mark: var(--prose-shade-apricot-page-45); }' }],
    [{ file: 'b.tsx', text: `<p class="text-ube dark:bg-celadon/40 text-ube-page" />; PALETTE['ube-pale']; PALETTE.whiskey` }]
  )

  it('reads a declaration, naming its property and selector and the scheme its selector applies in', () => {
    expect(uses.get('ube-deep')).toStrictEqual([{ schemes: palette.SCHEMES, token: 'color', where: 'a.css .x' }])
    expect(uses.get('ube')).toContainEqual({ schemes: ['dark'], token: 'border-color', where: 'a.css .dark .y' })
  })

  it('reads a declaration reading one of a color’s shades', () => {
    expect(uses.get('apricot')).toStrictEqual([{ schemes: palette.SCHEMES, token: '--mark', where: 'a.css .z' }])
  })

  it('reads a utility naming a whole color, its dark variant applying in the dark scheme alone', () => {
    expect(uses.get('ube')).toContainEqual({ schemes: palette.SCHEMES, token: 'text-ube', where: 'b.tsx' })
    expect(uses.get('celadon')).toStrictEqual([{ schemes: ['dark'], token: 'dark:bg-celadon', where: 'b.tsx' }])
  })

  it('reads an expression reading the palette in either form', () => {
    expect(uses.get('ube-pale')).toStrictEqual([{ schemes: palette.SCHEMES, token: 'PALETTE.ube-pale', where: 'b.tsx' }])
    expect(uses.get('whiskey')).toStrictEqual([{ schemes: palette.SCHEMES, token: 'PALETTE.whiskey', where: 'b.tsx' }])
  })
})

describe('swatches', () => {
  const all = swatches.swatches(new Map([['ube', [{ schemes: palette.SCHEMES, token: 'text-ube', where: 'b.tsx' }]]]), PAGE, new Map([['eureka', 'warning']]))

  it('sets out every palette color once, in the order of their groups', () => {
    expect(all.map(swatch => swatch.name).toSorted()).toStrictEqual(Object.keys(palette.PALETTE).toSorted())
    const order: readonly string[] = swatches.GROUPS
    expect(all.map(swatch => order.indexOf(swatch.group))).toStrictEqual(all.map(swatch => order.indexOf(swatch.group)).toSorted())
  })

  it('groups the brand hue with the shades mixed from it, darkest first, and records each mix', () => {
    const brand = all.filter(swatch => swatch.group === 'brand')
    expect(brand.map(swatch => swatch.name)).toStrictEqual([palette.BRAND, 'ube-night', 'ube-deep', 'ube-mid', 'ube-pale'])
    expect(brand.slice(1).map(swatch => swatch.mix)).toStrictEqual(['ube-night', 'ube-deep', 'ube-mid', 'ube-pale'].map(name => palette.BRAND_SHADES[name as keyof typeof palette.BRAND_SHADES]))
  })

  it('assigns each hue to the first declaration that takes it', () => {
    const group = (name: string) => all.find(swatch => swatch.name === name)!.group
    expect(group('eureka')).toBe('states')
    expect(group('apricot')).toBe('complements')
    expect(group('dexter')).toBe('sections')
    expect(group('woodsmoke')).toBe('neutrals')
    expect(group('champagne')).toBe('complements')
  })

  it('measures each color against the page of each scheme', () => {
    const ube = all.find(swatch => swatch.name === 'ube')!
    expect(ube.contrast.dark.ratio).toBe(wcagContrast(ube.hex, PAGE.dark))
  })

  it('lists the uses palette.ts declares before the scanned ones, each once', () => {
    const ube = all.find(swatch => swatch.name === 'ube')!
    expect(ube.uses).toContainEqual({ schemes: palette.SCHEMES, token: '--color-family-engine', where: 'palette.ts' })
    expect(ube.uses.at(-1)).toStrictEqual({ schemes: palette.SCHEMES, token: 'text-ube', where: 'b.tsx' })
    expect(new Set(ube.uses.map(use => JSON.stringify(use))).size).toBe(ube.uses.length)
  })
})
