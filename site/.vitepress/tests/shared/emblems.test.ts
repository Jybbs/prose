import fs from 'node:fs'

import { differenceCiede2000, interpolate, wcagContrast } from 'culori'
import { parse as parseCss }                              from 'postcss'

import * as emblems    from '../../lib/shared/emblems'
import * as palette    from '../../lib/shared/palette'
import * as registries from '../../lib/shared/registries'
import * as support    from '../support'

const FAMILIES_LISTED = Object.keys(registries.GLOSSARY_FAMILY_META) as registries.GlossaryFamily[]
const SMALL_DRAWINGS  = ['Apple', 'Basket', 'Harp', 'Newspaper', 'Owl', 'Penguin', 'Wand']

const tokens = support.declared(new URL('../../theme/styles/tokens.css', import.meta.url), ':root')
const share  = (token: string): number => parseFloat(tokens[token]) / 100

interface Scheme {
  bg    : string
  brand : string
  text  : string
}

// Reads the page ground the palette declares and the text color VitePress
// declares for one scheme, beside the brand color `tokens.css` sets for it.
function scheme(brand: string, name: 'dark' | 'light'): Scheme {
  const selector = name === 'dark' ? '.dark' : ':root'
  return {
    bg    : palette.GROUNDS.page[name],
    brand : brand,
    text  : support.vitepressVar('--vp-c-text-1', selector)
  }
}

const SCHEMES = {
  dark  : scheme(palette.PALETTE['ube-pale'], 'dark'),
  light : scheme(palette.PALETTE.ube, 'light')
}

// Mixes the way `color-mix(in srgb, hue <share>, base)` does.
const mix = (hue: string, base: string, amount: number) => interpolate([base, hue], 'rgb')(amount)

describe('emblemsOf', () => {
  const map = emblems.emblemsOf([{ emoji: '🐧', name: 'Linux' }, { emoji: '🍎', name: 'macOS' }])

  it('keys every family badge and platform emoji on its emblem', () => {
    expect([...map.keys()]).toStrictEqual([
      ...FAMILIES_LISTED.map(family => registries.GLOSSARY_FAMILY_META[family].badge), '🐧', '🍎'
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
      .toMatchObject({ family, name: registries.GLOSSARY_FAMILY_META[family].label })
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
  const ink     = share('--prose-tint-ink')
  const ground  = (colors: Scheme) => mix(colors.text, colors.bg, share('--prose-tint-faint'))
  const schemes = Object.entries(SCHEMES)
  const raised  = schemes.flatMap(([name, colors]) =>
    registries.FAMILY_ORDER.map(family => [name, family, colors] as const))

  it.each(schemes)('clears 3:1 at rest on the neutral ground in the %s scheme', (_scheme, colors) => {
    expect(wcagContrast(colors.text, ground(colors))).toBeGreaterThanOrEqual(3)
  })

  it.each(schemes)('clears 3:1 for a platform on the neutral ground in the %s scheme', (_, colors) => {
    expect(wcagContrast(mix(colors.brand, colors.text, ink), ground(colors))).toBeGreaterThanOrEqual(3)
  })

  it.each(raised)(
    'clears 3:1 raised on the page in the %s scheme for %s',
    (_scheme, family, colors) => {
      expect(wcagContrast(mix(palette.FAMILIES[family], colors.text, ink), colors.bg))
        .toBeGreaterThanOrEqual(3)
    }
  )

  it('applies the ink and the ground these cases measure', () => {
    const nib = new URL('../../theme/components/base/styles/nib.css', import.meta.url)
    expect(support.declared(nib, '.nib-badge').color)
      .toBe('color-mix(in srgb, var(--family-color) var(--prose-tint-ink), var(--vp-c-text-1))')
    const grounds = '.nib-chip,\n.nib-ground,\n.rule-chip,\n.rule-link'
    expect(support.declared(nib, grounds)['background-color'])
      .toBe('var(--prose-chip-ground)')
    expect(tokens['--prose-chip-ground'])
      .toBe('color-mix(in srgb, var(--vp-c-text-1) var(--prose-tint-faint), var(--vp-c-bg))')
  })

  it('takes its ink from nib.css alone, so no surface recolors a badge off its ground', () => {
    const recolored = support.siteSheets().filter(({ root }) => {
      let sets = false
      root.walkRules(rule => {
        if (rule.selector.includes('.nib-badge')) rule.walkDecls('color', () => { sets = true })
      })
      return sets
    })
    expect(recolored.map(sheet => sheet.file)).toStrictEqual(['.vitepress/theme/components/base/styles/nib.css'])
  })

  it('paints no ground under a hovered Pipeline Order row, whose badges stand on the page', () => {
    const css     = new URL('../../theme/components/rules/styles/pipeline-order.css', import.meta.url)
    const painted = [] as string[]
    parseCss(fs.readFileSync(css, 'utf8')).walkRules(rule => {
      if (rule.selector.includes(':hover')) rule.walkDecls(/^background/, () => { painted.push(rule.selector) })
    })
    expect(painted).toStrictEqual([])
  })

  it.each(schemes)('keeps each raised family an ink of its own in the %s scheme', (_scheme, colors) => {
    const inks = registries.FAMILY_ORDER.map(family => mix(palette.FAMILIES[family], colors.text, ink))
    const gaps = inks.flatMap((a, i) => inks.slice(i + 1).map(b => differenceCiede2000()(a, b)))
    expect(Math.min(...gaps)).toBeGreaterThan(5)
  })
})
