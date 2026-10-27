import { converter, formatHex, interpolate, parse, wcagContrast } from 'culori'
import { parse as parseCss }                                      from 'postcss'
import githubDark                                                 from 'shiki/themes/github-dark.mjs'

import * as palette from '../../lib/shared/palette'
import * as support from '../support'

const CHIPS   = ['danger', 'warning'] as const
const SCHEMES = ['dark', 'light'] as const

const lightness = (hex: string): number => converter('oklch')(parse(hex))!.l

// Mixes `share` percent of `hue` toward black the way `oklchMix` in the palette does.
const darker = (hue: string, share: number): string =>
  formatHex(interpolate([hue, 'black'], 'oklch')(share / 100))

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

  it('writes every group under :root and the dark scheme roles under .dark', () => {
    expect([...blocks.keys()]).toStrictEqual([':root', '.dark'])
    expect(blocks.get('.dark')!.map(([prop]) => prop))
      .toStrictEqual(Object.keys(palette.SCHEME_ROLES.dark).map(name => `--prose-role-${name}`))
  })

  it.each([...blocks.values()].flat())('%s is a concrete hex', (_name, value) => {
    expect(formatHex(parse(value))).toBe(value)
  })

  it.each([
    '--prose-family-cli',
    '--prose-palette-ube',
    '--prose-role-link',
    '--prose-role-squiggle',
    '--prose-section-guide',
    '--prose-shiki-dark-bg'
  ])('emits %s under its group prefix', name => {
    expect(blocks.get(':root')!.map(([prop]) => prop)).toContain(name)
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
})

describe('shadeReading', () => {
  const { apricot, toronto } = palette.PALETTE
  const { page }             = palette.GROUNDS

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
})

describe('SCHEME_ROLES', () => {
  it.each(SCHEMES)('reads the link at 4.5:1 on the %s page', scheme => {
    expect(wcagContrast(palette.SCHEME_ROLES[scheme].link, palette.GROUNDS.page[scheme]))
      .toBeGreaterThanOrEqual(4.5)
  })

  it.each(SCHEMES)('reads the squiggle at 3:1 on the %s code block', scheme => {
    expect(wcagContrast(palette.SCHEME_ROLES[scheme].squiggle, palette.GROUNDS.code[scheme]))
      .toBeGreaterThanOrEqual(3)
  })

  it.each(SCHEMES.flatMap(scheme => CHIPS.map(kind => [scheme, kind] as const)))(
    'sets the %s %s chip ink at 4.5:1 on its ground and its rule at 3:1 on the page',
    (scheme, kind) => {
      const roles = palette.SCHEME_ROLES[scheme]
      const page  = palette.GROUNDS.page[scheme]
      expect(wcagContrast(roles[`${kind}-ink`]!, roles[`${kind}-ground`]!)).toBeGreaterThanOrEqual(4.5)
      expect(wcagContrast(roles[`${kind}-rule`]!, page)).toBeGreaterThanOrEqual(3)
    }
  )
})

describe('the hues the roles mix', () => {
  it.each(SCHEMES)('grounds the %s warning and danger chips on 12%% of their hues', scheme => {
    const page  = palette.GROUNDS.page[scheme]
    const mixed = (hue: string) => formatHex(interpolate([page, hue], 'rgb')(0.12))
    expect(palette.SCHEME_ROLES[scheme]['warning-ground']).toBe(mixed(palette.PALETTE.eureka))
    expect(palette.SCHEME_ROLES[scheme]['danger-ground']).toBe(mixed(palette.PALETTE.apricot))
  })

  it.each(SCHEMES)('shades the %s squiggle from Apricot', scheme => {
    expect(palette.SCHEME_ROLES[scheme].squiggle)
      .toBe(palette.shadeReading(palette.GROUNDS.code[scheme], palette.PALETTE.apricot, 3))
  })

  it('reads the light code ground tokens.css sets', () => {
    const tokens = new URL('../../theme/styles/tokens.css', import.meta.url)
    expect(support.declared(tokens, ':root')['--vp-code-block-bg']).toBe('var(--prose-palette-grams-hair)')
    expect(palette.GROUNDS.code.light).toBe(palette.PALETTE['grams-hair'])
  })
})

describe('the sheets reading the roles', () => {
  const links = '.vp-doc a.body-link,\n.vp-doc a.tool-mark,\na.tool-mark'
  const code  = '.vp-doc .custom-block:is(.tip, .info, .warning, .danger) code'

  it.each([
    ['components/fixtures/styles/lint-findings.css', '.lint-flag',                    '--lint-color',    'var(--prose-role-squiggle)'],
    ['styles/markdown/body.css',                     links,                           'color',           'var(--prose-role-link)'],
    ['styles/markdown/callouts.css',                 '.callout-kind',                 'color',           'var(--callout-ink)'],
    ['styles/markdown/callouts.css',                 '.callout-mark',                 'color',           'var(--callout-ink)'],
    ['styles/markdown/callouts.css',                 '.vp-doc .custom-block.danger',  '--callout-ink',   'var(--prose-role-danger-ink)'],
    ['styles/markdown/callouts.css',                 '.vp-doc .custom-block.warning', '--callout-ink',   'var(--prose-role-warning-ink)'],
    ['styles/markdown/callouts.css',                 code,                            'color',           'var(--vp-code-color)'],
    ['styles/tokens.css',                            ':root',                         '--vp-code-color', 'var(--vp-c-text-1)']
  ])('%s sets %s %s to %s', (file, selector, prop, value) => {
    expect(support.declared(new URL(`../../theme/${file}`, import.meta.url), selector)[prop]).toBe(value)
  })
})
