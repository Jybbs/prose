import { formatHex, interpolate, wcagContrast, wcagLuminance } from 'culori'

import type { GlossaryFamily, SectionSlug } from './registries'

type Scheme = 'dark' | 'light'

// Blends `base` toward black or white the way `color-mix(in oklch, base, toward pct%)` does
const oklchMix = (base: string, toward: 'black' | 'white', share: number): string =>
  formatHex(interpolate([base, toward], 'oklch')(share / 100))

// Blends `hue` into `base` the way `color-mix(in srgb, hue pct%, base)` does
const srgbMix = (base: string, hue: string, share: number): string =>
  formatHex(interpolate([base, hue], 'rgb')(share / 100))

const HUES = {
  apricot      : '#e8876f',
  casper       : '#adbdcd',
  celadon      : '#8cc5a3',
  chambray     : '#7db3e0',
  champagne    : '#f0e9bc',
  dexter       : '#6db0b5',
  eureka       : '#e8c840',
  'grams-hair' : '#f6f8fa',
  heath        : '#c08597',
  oat          : '#cdbda5',
  rainee       : '#b8c8a8',
  toronto      : '#5069ad',
  ube          : '#8a80cb',
  whiskey      : '#d4a574',
  woodsmoke    : '#17171b'
} as const

export const PALETTE = {
  ...HUES,
  'ube-deep'  : oklchMix(HUES.ube, 'black', 22),
  'ube-mid'   : oklchMix(HUES.ube, 'white', 18),
  'ube-night' : oklchMix(HUES.ube, 'black', 45),
  'ube-pale'  : oklchMix(HUES.ube, 'white', 36)
}

// Holds the grounds each scheme draws on, the page being VitePress's
// `--vp-c-bg` and the code block the `--vp-code-block-bg` that `tokens.css`
// sets, Grams Hair in the light scheme and the `github-dark` theme's editor
// background in the dark.
export const GROUNDS: Record<'code' | 'page', Record<Scheme, string>> = {
  code : { dark: '#24292e', light: HUES['grams-hair'] },
  page : { dark: '#1b1b1f', light: '#ffffff' }
}

export const FAMILIES: Record<GlossaryFamily, string> = {
  alignment  : PALETTE.eureka,
  cli        : PALETTE['ube-night'],
  docs       : PALETTE.celadon,
  engine     : PALETTE.ube,
  formatting : PALETTE.heath,
  layout     : PALETTE.toronto,
  lint       : PALETTE.apricot,
  ordering   : PALETTE.chambray
}

const CALLOUTS = { danger: HUES.apricot, warning: HUES.eureka }
const LINKS    = { dark: PALETTE['ube-pale'], light: PALETTE['ube-deep'] }

// Keyed by section slug so a rename in the `SECTIONS` registry fails here,
// where the accent tokens the OG cards resolve are emitted. Rules pages
// accent by family instead, so the `rules` slug carries no section hue.
export const SECTIONS: Record<Exclude<SectionSlug, 'rules'>, string> = {
  architecture : PALETTE.dexter,
  guide        : PALETTE.oat,
  integrations : PALETTE.rainee,
  reference    : PALETTE.casper,
  sandbox      : PALETTE.whiskey
}

// Returns the lightest `oklchMix` of `hue` toward black, or toward white on a
// dark ground, that reads at `ratio` against `ground`, which is `hue` itself
// wherever the hue already reads at that ratio.
export function shadeReading(ground: string, hue: string, ratio: number): string {
  const toward = wcagLuminance(ground) > 0.5 ? 'black' : 'white'
  const shades = Array.from({ length: 101 }, (_, share) => share ? oklchMix(hue, toward, share) : hue)
  return shades.find(shade => wcagContrast(shade, ground) >= ratio)!
}

// Lists the roles whose color depends on the scheme. A warning or danger
// callout chip sits on 12% of its hue over the page, its ink reading 4.5:1 on
// that ground and its rule 3:1 on the page, and the lint squiggle reads 3:1 on
// the code block.
function schemeRoles(scheme: Scheme): Record<string, string> {
  const page = GROUNDS.page[scheme]
  const chip = Object.entries(CALLOUTS).flatMap(([kind, hue]) => {
    const ground = srgbMix(page, hue, 12)
    return [
      [`${kind}-ground`, ground],
      [`${kind}-ink`, shadeReading(ground, hue, 4.5)],
      [`${kind}-rule`, shadeReading(page, hue, 3)]
    ]
  })
  return {
    ...Object.fromEntries(chip),
    link     : LINKS[scheme],
    squiggle : shadeReading(GROUNDS.code[scheme], HUES.apricot, 3)
  }
}

export const SCHEME_ROLES: Record<Scheme, Record<string, string>> = {
  dark  : schemeRoles('dark'),
  light : schemeRoles('light')
}

const GROUPS = {
  family  : FAMILIES,
  palette : PALETTE,
  role    : SCHEME_ROLES.light,
  section : SECTIONS,
  shiki   : { 'dark-bg': GROUNDS.code.dark }
}

// Writes each group's colors as `--prose-<group>-<name>` properties under one
// selector.
function block(groups: Record<string, Record<string, string>>, selector: string): string {
  const lines = Object.entries(groups).flatMap(([prefix, entries]) =>
    Object.entries(entries).map(([name, value]) => `  --prose-${prefix}-${name}: ${value};`))
  return `${selector} {\n${lines.join('\n')}\n}\n`
}

export function paletteCss(): string {
  return block(GROUPS, ':root') + block({ role: SCHEME_ROLES.dark }, '.dark')
}
