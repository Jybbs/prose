import fs from 'node:fs'

import { differenceCiede2000, interpolate, wcagContrast } from 'culori'
import { parse as parseCss }                              from 'postcss'

import * as emblems                                  from '../../lib/shared/emblems'
import { FAMILIES, PALETTE }                         from '../../lib/shared/palette'
import { GLOSSARY_FAMILY_META, type GlossaryFamily } from '../../lib/shared/registries'

const FAMILIES_LISTED = Object.keys(GLOSSARY_FAMILY_META) as GlossaryFamily[]
const SMALL_DRAWINGS  = ['Apple', 'Basket', 'Harp', 'Newspaper', 'Owl', 'Penguin', 'Wand']

// Reads each declaration a stylesheet makes under `selector`.
function declared(file: string | URL, selector: string): Record<string, string> {
  const props: Record<string, string> = {}
  parseCss(fs.readFileSync(file, 'utf8')).walkRules(rule => {
    if (rule.selector !== selector) return
    rule.walkDecls(decl => { props[decl.prop] = decl.value })
  })
  return props
}

const VITEPRESS = new URL('../../../node_modules/vitepress/dist/client/theme-default/styles/vars.css', import.meta.url)
const tokens    = declared(new URL('../../theme/styles/tokens.css', import.meta.url), ':root')
const share     = (token: string): number => parseFloat(tokens[token]) / 100

interface Scheme {
  bg    : string
  brand : string
  text  : string
}

// Reads the page and text colors VitePress declares for one scheme beside the
// brand color `tokens.css` sets for it.
function scheme(selector: string, brand: string): Scheme {
  const vars = declared(VITEPRESS, selector)
  return { bg: vars['--vp-c-bg'], brand, text: vars['--vp-c-text-1'] }
}

const SCHEMES = { dark: scheme('.dark', PALETTE['ube-pale']), light: scheme(':root', PALETTE.ube) }

// Mixes the way `color-mix(in srgb, hue <share>, base)` does.
const mix = (hue: string, base: string, amount: number) => interpolate([base, hue], 'rgb')(amount)

describe('emblemsOf', () => {
  const map = emblems.emblemsOf([{ emoji: '🐧', name: 'Linux' }, { emoji: '🍎', name: 'macOS' }])

  it('keys every family badge and platform emoji on its emblem', () => {
    expect([...map.keys()]).toStrictEqual([
      ...FAMILIES_LISTED.map(family => GLOSSARY_FAMILY_META[family].badge), '🐧', '🍎'
    ])
  })

  it('names a platform emblem after the platform and gives it no family', () => {
    expect(map.get('🍎')).toMatchObject({ motif: { label: 'Apple' }, name: 'macOS' })
    expect(map.get('🍎')!.family).toBeUndefined()
  })

  it('fails on a platform no motif draws', () => {
    expect(() => emblems.emblemsOf([{ emoji: '🐡', name: 'Fugu' }])).toThrow(/Platform "Fugu"/)
  })
})

describe('familyEmblem', () => {
  it.each(FAMILIES_LISTED)('names the %s emblem after its family label', family => {
    expect(emblems.familyEmblem(family))
      .toMatchObject({ family, name: GLOSSARY_FAMILY_META[family].label })
  })
})

describe('MOTIFS', () => {
  it('gives a small drawing to each motif that loses its structure below the large size', () => {
    expect(emblems.MOTIFS.filter(motif => motif.small).map(motif => motif.label).toSorted())
      .toStrictEqual(SMALL_DRAWINGS)
  })

  it('draws the wand\'s small shaft as its one heavy stroke', () => {
    expect(emblems.MOTIFS.filter(motif => motif.small?.heavy).map(motif => motif.label))
      .toStrictEqual(['Wand'])
  })
})

describe('the badge ink', () => {
  const ink    = share('--prose-tint-ink')
  const ground = share('--prose-tint-ground')

  const cases = Object.entries(SCHEMES).flatMap(([name, colors]) => [
    ...FAMILIES_LISTED.map(family => [name, family, FAMILIES[family], colors] as const),
    [name, 'platform', colors.brand, colors] as const
  ])

  it.each(cases)('clears 3:1 on its ground in the %s scheme for %s', (_scheme, _who, hue, colors) => {
    const contrast = wcagContrast(mix(hue, colors.text, ink), mix(hue, colors.bg, ground))
    expect(contrast).toBeGreaterThanOrEqual(3)
  })

  it('applies the ink and the ground these cases measure', () => {
    const nib = new URL('../../theme/components/base/styles/nib.css', import.meta.url)
    const hue = 'var(--family-color, var(--vp-c-brand-1))'
    expect(declared(nib, '.nib-badge').color)
      .toBe(`color-mix(in srgb, ${hue} var(--prose-tint-ink), var(--vp-c-text-1))`)
    expect(declared(nib, '.nib-chip,\n.nib-ground,\n.rule-chip,\n.rule-link')['background-color'])
      .toBe(`color-mix(in srgb, ${hue} var(--prose-tint-ground), var(--vp-c-bg))`)
  })

  it.each(Object.entries(SCHEMES))('keeps each family an ink of its own in the %s scheme', (_scheme, colors) => {
    const inks = FAMILIES_LISTED.map(family => mix(FAMILIES[family], colors.text, ink))
    const gaps = inks.flatMap((a, i) => inks.slice(i + 1).map(b => differenceCiede2000()(a, b)))
    expect(Math.min(...gaps)).toBeGreaterThan(5)
  })
})
