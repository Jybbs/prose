import { differenceCiede2000, interpolate, wcagContrast } from 'culori'
import { parse as parseCss }                              from 'postcss'

import * as emblems    from '../../lib/shared/emblems'
import * as palette    from '../../lib/shared/palette'
import * as registries from '../../lib/shared/registries'
import * as support    from '../support'
import * as tailwind   from '../tailwind'
import * as vocabulary from '../vocabulary'

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
function scheme(brand: string, name: palette.Scheme): Scheme {
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
  const ground  = (colors: Scheme) => mix(colors.text, colors.bg, share('--prose-tint-faint'))
  const schemes = Object.entries(SCHEMES)
  const NIB_INK = /^var\(--(?:family-(?:nib|edge)(?:-card|-chip|-soft)?|prose-shade-text-3-[a-z]+-31|vp-c-text-2)\)$/

  it.each(schemes)('clears 3:1 for a platform on the neutral ground in the %s scheme', (_, colors) => {
    expect(wcagContrast(mix(colors.brand, colors.text, share('--prose-tint-ink')), ground(colors))).toBeGreaterThanOrEqual(3)
  })

  it('draws a badge in its family\'s nib, a platform badge in the brand ink', () => {
    const nib      = new URL('../../theme/components/base/styles/nib.css', import.meta.url)
    expect(support.declared(nib, '.nib-badge')['--nib-platform'])
      .toBe('color-mix(in srgb, var(--vp-c-brand-1) var(--prose-tint-ink), var(--vp-c-text-1))')
    expect(support.declared(nib, '.nib-badge').color).toBe('var(--family-nib, var(--nib-platform))')
    expect(support.declared(nib, ':is(.nib-chip, .nib-ground, .rule-chip, .rule-link) .nib-badge').color)
      .toBe('var(--family-nib-chip, var(--nib-platform))')
    expect(tokens['--prose-chip-ground'])
      .toBe('color-mix(in srgb, var(--vp-c-text-1) var(--prose-tint-faint), var(--vp-c-bg))')
  })

  it('recolors a badge only in a family shade for its ground or a quiet gray', () => {
    const strays = support.siteSheets().flatMap(({ file, root }) => {
      const found = [] as string[]
      root.walkRules(rule => {
        if (!rule.selector.includes('.nib-badge') || file.endsWith('base/styles/nib.css')) return
        rule.walkDecls('color', decl => { if (!NIB_INK.test(decl.value)) found.push(`${file} ${rule.selector} ${decl.value}`) })
      })
      return found
    })
    expect(strays).toStrictEqual([])
  })

  it('paints no ground under a hovered Pipeline Order row, whose badges stand on the page', async () => {
    const design  = await tailwind.designSystem()
    const painted = [] as string[]
    parseCss(vocabulary.read('.vitepress/theme/components/rules/styles/pipeline-order.css')).walkRules(rule => {
      if (rule.selector.includes(':hover')) rule.walkDecls(/^background/, () => { painted.push(rule.selector) })
    })
    const template = vocabulary.read('.vitepress/theme/components/rules/PipelineOrder.vue')
    for (const [name] of template.matchAll(/[\w/-]*hover(?:\/[\w-]+)?:[\w/-]+/g)) {
      if (/background/.test(design.candidatesToCss([name])[0] ?? '')) painted.push(name)
    }
    expect(painted).toStrictEqual([])
  })

  it.each(palette.SCHEMES)('keeps each family\'s nib a shade of its own in the %s scheme', mode => {
    const nibs = registries.FAMILY_ORDER.map(family => palette.SCHEME_SHADES[mode][`${family}-page-45`]!)
    const gaps = nibs.flatMap((a, i) => nibs.slice(i + 1).map(b => differenceCiede2000()(a, b)))
    expect(Math.min(...gaps)).toBeGreaterThan(5)
  })
})
