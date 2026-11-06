import fs   from 'node:fs'
import path from 'node:path'

import { converter, formatHex, interpolate, parse, wcagContrast } from 'culori'
import { parse as parseCss, type AtRule }                         from 'postcss'
import githubDark                                                 from 'shiki/themes/github-dark.mjs'
import githubLight                                                from 'shiki/themes/github-light.mjs'

import { SHIKI_COLOR_REPLACEMENTS } from '../../lib/shared/constants'
import * as palette                from '../../lib/shared/palette'
import * as support                from '../support'

const SCHEMES = ['dark', 'light'] as const

const lightness = (hex: string): number => converter('oklch')(parse(hex))!.l

// Mixes `share` percent of `hue` toward black the way `oklchMix` in the palette does.
const darker = (hue: string, share: number): string =>
  formatHex(interpolate([hue, 'black'], 'oklch')(share / 100))

// Mixes `share` percent of `hue` into `base` the way `color-mix(in srgb)` does.
const over = (base: string, hue: string, share: number): string =>
  formatHex(interpolate([base, hue], 'rgb')(share / 100))

// Reads each declaration `paletteCss` writes, keyed by the selector it sits under.
function declarations(): Map<string, [string, string][]> {
  const blocks = new Map<string, [string, string][]>()
  parseCss(palette.paletteCss()).walkDecls(decl => {
    const selector = (decl.parent as { selector: string }).selector
    blocks.set(selector, [...blocks.get(selector) ?? [], [decl.prop, decl.value]])
  })
  return blocks
}

describe('paletteCss', () => {
  const blocks = declarations()

  it('writes the shades under :root and .dark, beside each family\'s accents', () => {
    expect([...blocks.keys()]).toStrictEqual([':root', '.dark', ...Object.keys(palette.FAMILIES).map(family =>
      `[data-family="${family}"],\n[data-popup="rule-card"]:has(.rule-card[data-family="${family}"])`)])
    expect(blocks.get('.dark')!.map(([prop]) => prop))
      .toStrictEqual(Object.keys(palette.SCHEME_SHADES.dark).map(name => `--prose-shade-${name}`))
  })

  it.each([...blocks.get(':root')!, ...blocks.get('.dark')!])('%s is a concrete hex', (_name, value) => {
    expect(formatHex(parse(value))).toBe(value)
  })

  it.each([
    '--prose-shade-alignment-chip-45',
    '--prose-shade-text-3-code-35',
    '--prose-shiki-dark-bg'
  ])('emits %s under its group prefix', name => {
    expect(blocks.get(':root')!.map(([prop]) => prop)).toContain(name)
  })
})

describe('themeCss', () => {
  const root   = parseCss(palette.themeCss())
  const colors = new Map<string, string>()
  const dark   = new Map<string, string>()
  root.walkAtRules('theme', theme => { theme.walkDecls(decl => { colors.set(decl.prop, decl.value) }) })
  root.walkAtRules('layer', layer => { layer.walkDecls(decl => { dark.set(decl.prop, decl.value) }) })

  it('declares the colors in a `@theme static` block and the dark roles in the theme layer', () => {
    expect(root.nodes.map(node => (node as AtRule).params)).toStrictEqual(['static', 'theme'])
    expect([...dark.keys()]).toStrictEqual(Object.keys(palette.SCHEME_ROLES.dark).map(name => `--color-${name}`))
  })

  it.each([...colors, ...dark])('%s is a concrete hex', (_name, value) => {
    expect(formatHex(parse(value))).toBe(value)
  })

  it.each([
    ['--color-family-cli',    palette.FAMILIES.cli],
    ['--color-link',          palette.SCHEME_ROLES.light.link],
    ['--color-section-guide', palette.SECTIONS.guide],
    ['--color-ube',           palette.PALETTE.ube]
  ])('declares %s from its source', (name, hex) => {
    expect(colors.get(name)).toBe(hex)
  })
})

describe('PALETTE', () => {
  it('blends distinct shares to distinct colors', () => {
    expect(palette.PALETTE['ube-pale']).not.toBe(palette.PALETTE['ube-mid'])
  })

  it.each([
    ['ube-deep',  'darker'],
    ['ube-night', 'darker'],
    ['ube-mid',   'lighter'],
    ['ube-pale',  'lighter']
  ] as const)('mixes %s %s than the base hue', (shade, direction) => {
    const delta = lightness(palette.PALETTE[shade]) - lightness(palette.PALETTE.ube)
    expect(direction === 'darker' ? delta : -delta).toBeLessThan(0)
  })
})

describe('GROUNDS', () => {
  it.each([['dark', '.dark'], ['light', ':root']] as const)(
    'reads the %s page ground VitePress declares',
    (scheme, selector) => {
      expect(palette.GROUNDS.page[scheme]).toBe(support.vitepressVar('--vp-c-bg', selector))
    }
  )

  it('reads the dark code ground the github-dark theme sets', () => {
    expect(palette.GROUNDS.code.dark).toBe(githubDark.colors?.['editor.background'])
  })

  it('reads the grounds tokens.css sets for code and the dark soft ground', () => {
    const tokens = new URL('../../theme/styles/tokens.css', import.meta.url)
    expect(support.declared(tokens, ':root')['--vp-code-block-bg']).toBe('var(--color-grams-hair)')
    expect(support.declared(tokens, '.dark')['--vp-c-bg-soft']).toBe('var(--color-woodsmoke)')
    expect(palette.GROUNDS.code.light).toBe(palette.PALETTE['grams-hair'])
    expect(palette.GROUNDS.soft.dark).toBe(palette.PALETTE.woodsmoke)
  })

  it('reads the light soft ground VitePress declares', () => {
    expect(palette.GROUNDS.soft.light).toBe(support.vitepressVar('--vp-c-bg-soft', ':root'))
  })
})

describe('VITEPRESS', () => {
  it.each(SCHEMES.flatMap(scheme => (['bg-alt', 'text-1', 'text-2', 'text-3'] as const).map(name => [scheme, name] as const)))(
    'reads the %s %s VitePress declares',
    (scheme, name) => {
      expect(palette.VITEPRESS[name][scheme]).toBe(support.vitepressVar(`--vp-c-${name}`, scheme === 'dark' ? '.dark' : ':root'))
    }
  )

  it.each(SCHEMES)('lays the %s gray-soft ground over the page at its alpha', scheme => {
    const [r, g, b, alpha] = support.vitepressVar('--vp-c-gray-soft', scheme === 'dark' ? '.dark' : ':root')
      .match(/[\d.]+/g)!.map(Number) as [number, number, number, number]
    const hue = formatHex({ b: b / 255, g: g / 255, mode: 'rgb', r: r / 255 })
    expect(palette.VITEPRESS['gray-soft'][scheme]).toBe(over(palette.GROUNDS.page[scheme], hue, alpha * 100))
  })
})

describe('shadeReading', () => {
  const { apricot, toronto } = palette.PALETTE
  const { page, soft }       = palette.GROUNDS

  it('returns the hue itself where it already reads at the ratio', () => {
    expect(palette.shadeReading(page.dark, apricot, 3)).toBe(apricot)
  })

  it('returns the lightest shade reaching the ratio, the share before it falling short', () => {
    const shade = palette.shadeReading(page.light, apricot, 4.5)
    const share = Array.from({ length: 100 }, (_, i) => i + 1).find(i => darker(apricot, i) === shade)!
    expect(wcagContrast(shade, page.light)).toBeGreaterThanOrEqual(4.5)
    expect(wcagContrast(darker(apricot, share - 1), page.light)).toBeLessThan(4.5)
  })

  it('lightens toward white on a dark ground', () => {
    expect(lightness(palette.shadeReading(page.dark, toronto, 4.5))).toBeGreaterThan(lightness(toronto))
  })

  it('reads at the ratio on every ground it names', () => {
    const shade = palette.shadeReading([page.light, soft.light], apricot, 4.5)
    expect(Math.min(wcagContrast(shade, page.light), wcagContrast(shade, soft.light))).toBeGreaterThanOrEqual(4.5)
    expect(shade).not.toBe(palette.shadeReading(page.light, apricot, 4.5))
  })
})

describe('SCHEME_ROLES', () => {
  it.each(SCHEMES)('reads the %s link at 4.5:1 on the page', scheme => {
    expect(wcagContrast(palette.SCHEME_ROLES[scheme].link!, palette.GROUNDS.page[scheme]))
      .toBeGreaterThanOrEqual(4.5)
  })

  it.each([['light', 9.7], ['dark', 10.1]] as const)(
    'reads the %s aside ink near %d:1 on the page, below the body text',
    (scheme, ratio) => {
      const page  = palette.GROUNDS.page[scheme]
      const aside = wcagContrast(palette.SCHEME_ROLES[scheme].aside!, page)
      expect(aside).toBeCloseTo(ratio, 1)
      expect(aside).toBeLessThan(wcagContrast(palette.VITEPRESS['text-1'][scheme], page))
    }
  )

  it.each(SCHEMES)('turns the %s gray code ground to Ube\'s hue at nearly its lightness', scheme => {
    const toOklch = converter('oklch')
    const ground  = toOklch(palette.SCHEME_ROLES[scheme]['code-ground'])!
    expect(Math.abs(ground.h! - toOklch(palette.PALETTE.ube)!.h!)).toBeLessThan(2)
    expect(ground.l).toBeCloseTo(toOklch(palette.VITEPRESS['gray-soft'][scheme])!.l, 1)
  })
})

describe('SCHEME_SHADES', () => {
  const chip = (scheme: 'dark' | 'light') => over(palette.GROUNDS.page[scheme], palette.VITEPRESS['text-1'][scheme], 5)

  it.each(SCHEMES.flatMap(scheme => Object.entries(palette.SCHEME_SHADES[scheme]).map(([name, hex]) => [scheme, name, hex] as const)))(
    'reads the %s %s at its ratio on its ground',
    (scheme, name, hex) => {
      const [, ground, digits] = /-(any|card|chip|code|page|selected|soft|wash)-(\d+)$/.exec(name)!
      const grounds = {
        any  : [palette.GROUNDS.page[scheme], palette.GROUNDS.soft[scheme], palette.GROUNDS.code[scheme]],
        card : [palette.GROUNDS.page[scheme], palette.GROUNDS.soft[scheme]],
        chip : [chip(scheme)],
        code : [palette.GROUNDS.code[scheme]],
        page : [palette.GROUNDS.page[scheme]],
        selected : [over(palette.GROUNDS.page[scheme], palette.PALETTE.ube, 12)],
        soft : [palette.GROUNDS.soft[scheme]],
        wash : [over(palette.GROUNDS.page[scheme], palette.VITEPRESS['bg-alt'][scheme], 30)]
      }[ground!]!
      const ratio = Number(digits) / 10
      for (const each of grounds) expect(wcagContrast(hex, each)).toBeGreaterThanOrEqual(ratio)
    }
  )

  it('shades every family on every ground a nib or an edge reads on', () => {
    for (const family of Object.keys(palette.FAMILIES)) {
      for (const name of ['card', 'chip', 'page', 'soft'].flatMap(ground => [`${ground}-34`, `${ground}-45`])) {
        expect(palette.SCHEME_SHADES.light).toHaveProperty(`${family}-${name}`)
      }
    }
  })
})

describe('SHIKI_COLOR_REPLACEMENTS', () => {
  it.each([
    ['dark',  githubDark],
    ['light', githubLight]
  ] as const)('reads every token color of the %s theme at 4.5:1 on the code block', (scheme, theme) => {
    const replacements: Record<string, string> = SHIKI_COLOR_REPLACEMENTS[theme.name as keyof typeof SHIKI_COLOR_REPLACEMENTS]
    const foregrounds = theme.tokenColors!
      .filter(({ settings }) => !settings.background)
      .flatMap(({ settings }) => settings.foreground?.toLowerCase().slice(0, 7) ?? [])
    const painted     = [...new Set(foregrounds)].map(color => replacements[color] ?? color)
    expect(painted.filter(color => wcagContrast(color, palette.GROUNDS.code[scheme]) < 4.5)).toStrictEqual([])
  })

  it.each([
    ['dark',  'github-dark'],
    ['light', 'github-light']
  ] as const)('reads every replacement of the %s map at 4.5:1 on the code block', (scheme, theme) => {
    const values = Object.values(SHIKI_COLOR_REPLACEMENTS[theme])
    expect(values.filter(color => wcagContrast(color, palette.GROUNDS.code[scheme]) < 4.5)).toStrictEqual([])
  })
})

describe('the sheets reading the roles and shades', () => {
  const code = '.vp-doc .custom-block:is(.tip, .info, .warning, .danger) code'

  it.each([
    ['components/fixtures/styles/lint-findings.css', '.lint-flag',                    '--lint-color',    'var(--prose-shade-apricot-code-35)'],
    ['styles/markdown/callouts.css',                 '.vp-doc .custom-block.danger',  '--callout-mark',  'var(--prose-shade-apricot-page-45)'],
    ['styles/markdown/callouts.css',                 '.vp-doc .custom-block.tip',     '--callout-mark',  'var(--color-link)'],
    ['styles/markdown/callouts.css',                 '.vp-doc .custom-block.warning', '--callout-mark',  'var(--prose-shade-eureka-page-45)'],
    ['styles/markdown/callouts.css',                 code,                            'color',           'var(--vp-code-color)'],
    ['styles/tokens.css',                            ':root',                         '--vp-code-bg',    'var(--color-code-ground)'],
    ['styles/tokens.css',                            ':root',                         '--vp-code-color', 'var(--vp-c-text-1)']
  ])('%s sets %s %s to %s', (file, selector, prop, value) => {
    expect(support.declared(new URL(`../../theme/${file}`, import.meta.url), selector)[prop]).toBe(value)
  })
})

describe('tokens.css', () => {
  const css = (file: string) => fs.readFileSync(path.resolve(import.meta.dirname, '../../theme/styles', file), 'utf8')

  it('reads no `@theme inline` name, which Tailwind leaves undeclared at runtime', () => {
    const inline = new Set(css('tailwind/base.css').matchAll(/^ +(--color-[\w-]+) +:/gm).map(([, name]) => name))
    const read   = [...css('tokens.css').matchAll(/var\((--color-[\w-]+)\)/g)].map(([, name]) => name)
    expect(read.filter(name => inline.has(name))).toStrictEqual([])
  })
})
