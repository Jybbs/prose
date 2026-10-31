import fs   from 'node:fs'
import path from 'node:path'

import { differenceCiede2000, interpolate, wcagContrast } from 'culori'
import { parse as parseCss }                              from 'postcss'

import * as emblems                                  from '../../lib/shared/emblems'
import { FAMILIES, PALETTE }                         from '../../lib/shared/palette'
import { GLOSSARY_FAMILY_META, type GlossaryFamily } from '../../lib/shared/registries'
import { siteSheets }                                from '../support'
import * as tailwind                                 from '../tailwind'
import * as vocabulary                               from '../vocabulary'

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

// Reads each declaration the shapes include makes directly under `@utility name`.
function shape(name: string): Record<string, string> {
  const props: Record<string, string> = {}
  parseCss(fs.readFileSync(SHAPES, 'utf8')).walkAtRules('utility', rule => {
    if (rule.params !== name) return
    rule.each(node => { if (node.type === 'decl') props[node.prop] = node.value })
  })
  return props
}

const SHAPES    = new URL('../../theme/styles/tailwind/shapes.css', import.meta.url)
const VITEPRESS = path.join(vocabulary.SITE, vocabulary.VITEPRESS_THEME, 'styles/vars.css')
const tokens    = declared(path.join(vocabulary.SITE, vocabulary.TOKENS), ':root')
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
    const hue = 'var(--family-color, var(--vp-c-brand-1))'
    expect(shape('nib-badge').color)
      .toBe(`color-mix(in srgb, ${hue} var(--prose-tint-ink), var(--vp-c-text-1))`)
    expect(shape('ground')['background-color'])
      .toBe(`color-mix(in srgb, ${hue} var(--prose-tint-ground), var(--vp-c-bg))`)
  })

  it('takes its ink from the nib-badge shape alone, so no surface recolors a badge off its ground', () => {
    const recolored = siteSheets().filter(({ root }) => {
      let sets = false
      root.walkRules(rule => {
        if (rule.selector.includes('.nib-badge')) rule.walkDecls('color', () => { sets = true })
      })
      return sets
    })
    expect(recolored.map(sheet => sheet.file)).toStrictEqual([])
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

  it.each(Object.entries(SCHEMES))('keeps each family an ink of its own in the %s scheme', (_scheme, colors) => {
    const inks = FAMILIES_LISTED.map(family => mix(FAMILIES[family], colors.text, ink))
    const gaps = inks.flatMap((a, i) => inks.slice(i + 1).map(b => differenceCiede2000()(a, b)))
    expect(Math.min(...gaps)).toBeGreaterThan(5)
  })
})
