import { converter, formatHex, interpolate, wcagContrast, wcagLuminance } from 'culori'

import type { GlossaryFamily, SectionSlug } from './registries'

export type Scheme = (typeof SCHEMES)[number]

type Ground = keyof typeof GROUNDS | 'any' | 'card' | 'chip' | 'selected' | 'wash'
type Text   = 'text-1' | 'text-2' | 'text-3'

export const SCHEMES = ['dark', 'light'] as const

// Builds a record holding what `read` returns for each scheme.
export const perScheme = <T>(read: (scheme: Scheme) => T): Record<Scheme, T> =>
  Object.fromEntries(SCHEMES.map(scheme => [scheme, read(scheme)])) as Record<Scheme, T>

const oklch = converter('oklch')

// Blends `base` toward black or white the way `color-mix(in oklch, base, toward pct%)` does
const oklchMix = (base: string, toward: 'black' | 'white', share: number): string =>
  formatHex(interpolate([base, toward], 'oklch')(share / 100))

// Blends `hue` into `base` the way `color-mix(in srgb, hue pct%, base)` does
export const srgbMix = (base: string, hue: string, share: number): string =>
  formatHex(interpolate([base, hue], 'rgb')(share / 100))

export const HUES = {
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

export const BRAND = 'ube' satisfies keyof typeof HUES

// Declares each shade of the brand hue as the color it mixes toward and the
// share it takes of that color.
export const BRAND_SHADES = {
  'ube-deep'  : { share: 22, toward: 'black' },
  'ube-mid'   : { share: 18, toward: 'white' },
  'ube-night' : { share: 45, toward: 'black' },
  'ube-pale'  : { share: 36, toward: 'white' }
} as const satisfies Record<string, { share: number, toward: 'black' | 'white' }>

export const PALETTE = {
  ...HUES,
  ...Object.fromEntries(Object.entries(BRAND_SHADES).map(([name, { share, toward }]) =>
    [name, oklchMix(HUES[BRAND], toward, share)])) as Record<keyof typeof BRAND_SHADES, string>
}

// Holds the grounds each scheme draws on, the page being VitePress's
// `--vp-c-bg`, the soft ground its `--vp-c-bg-soft`, which `tokens.css` sets in
// the dark, and the code block the `--vp-code-block-bg` that `tokens.css` sets,
// Grams Hair in the light scheme and the `github-dark` theme's editor background
// in the dark.
export const GROUNDS = {
  code : { dark: '#24292e', light: HUES['grams-hair'] },
  page : { dark: '#1b1b1f', light: '#ffffff' },
  soft : { dark: HUES.woodsmoke, light: '#f6f6f7' }
} satisfies Record<string, Record<Scheme, string>>

// Holds VitePress's three text colors and its `--vp-c-bg-alt` and
// `--vp-c-gray-soft` grounds, the last as the opaque color its alpha leaves
// over the page.
export const VITEPRESS: Record<Text | 'bg-alt' | 'gray-soft', Record<Scheme, string>> = {
  'bg-alt'    : { dark: '#161618', light: '#f6f6f7' },
  'gray-soft' : { dark: srgbMix(GROUNDS.page.dark, '#657585', 16), light: srgbMix(GROUNDS.page.light, '#8e96aa', 14) },
  'text-1'    : { dark: '#dfdfd6', light: '#3c3c43' },
  'text-2'    : { dark: '#98989f', light: '#67676c' },
  'text-3'    : { dark: '#6a6a71', light: '#929295' }
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

type Hue = GlossaryFamily | keyof typeof PALETTE | Text

// Holds the contrast ratio each family accent reads at, the edge marking a
// border and the nib inking a badge.
export const ACCENT_RATIOS = { edge: 3.4, nib: 4.5 } as const

// Lists the shades the site reads, each row naming the colors it shades, the
// grounds they sit on, and the contrast ratios they read at. The `card` ground
// is a fixture card's, which shows the soft ground closed and the page open,
// the `chip` ground is 5% of the text color over the page, the `selected` ground
// is 12% of Ube over the page, the `wash` ground is 30% of `--vp-c-bg-alt` over
// the page, and the `any` ground is each of the page, the soft ground, and the
// code block.
const SHADES: [colors: readonly Hue[], grounds: readonly Ground[], ratios: readonly number[]][] = [
  [Object.keys(FAMILIES) as GlossaryFamily[], ['card', 'chip', 'page', 'soft'], Object.values(ACCENT_RATIOS)],
  [['apricot', 'celadon', 'eureka'], ['page'], [3.5, 4.5]],
  [['celadon'], ['page'], [6.5]],
  [['apricot'], ['code'], [3.5]],
  [['dexter', 'ube', 'whiskey'], ['page'], [4.5]],
  [['celadon', 'chambray', 'ube', 'whiskey'], ['wash'], [3.5, 4.5]],
  [['dexter', 'ube', 'whiskey'], ['code', 'soft'], [3.5]],
  [['text-3'], ['card', 'chip', 'soft'], [3.1]],
  [['text-3'], ['code'], [3.5]],
  [['text-3'], ['any'], [4.5]],
  [['apricot', 'celadon'], ['code', 'selected'], [4.5]],
  [['celadon', 'ube'], ['soft'], [4.5]],
  [['ube'], ['card'], [4.5]]
]

// Returns the `oklchMix` of `hue` nearest the hue that reads at `ratio`
// against every one of `grounds`, mixing toward black on a light ground and
// toward white on a dark one, which is `hue` itself wherever the hue already
// reads at that ratio.
export function shadeReading(grounds: string | readonly string[], hue: string, ratio: number): string {
  const all    = [grounds].flat()
  const toward = wcagLuminance(all[0]!) > 0.5 ? 'black' : 'white'
  const shades = Array.from({ length: 101 }, (_, share) => share ? oklchMix(hue, toward, share) : hue)
  return shades.find(shade => all.every(ground => wcagContrast(shade, ground) >= ratio))!
}

// Returns the grounds a shade on `ground` reads against in `scheme`.
function groundsOf(ground: Ground, scheme: Scheme): string[] {
  const page = GROUNDS.page[scheme]
  switch (ground) {
    case 'any'      : return [page, GROUNDS.soft[scheme], GROUNDS.code[scheme]]
    case 'card'     : return [page, GROUNDS.soft[scheme]]
    case 'chip'     : return [srgbMix(page, VITEPRESS['text-1'][scheme], 5)]
    case 'selected' : return [srgbMix(page, PALETTE.ube, 12)]
    case 'wash'     : return [srgbMix(page, VITEPRESS['bg-alt'][scheme], 30)]
    default         : return [GROUNDS[ground][scheme]]
  }
}

// Returns the hex `hue` names in `scheme`.
function hexOf(hue: Hue, scheme: Scheme): string {
  if (hue in FAMILIES) return FAMILIES[hue as GlossaryFamily]
  if (hue in PALETTE)  return PALETTE[hue as keyof typeof PALETTE]
  return VITEPRESS[hue as Text][scheme]
}

// Names a shade `<hue>-<ground>-<ratio>`, the ratio without its point, as
// `alignment-chip-45` for the alignment hue reading 4.5:1 on the chip ground.
export function shadeName(hue: string, ground: string, ratio: number): string {
  return `${hue}-${ground}-${String(ratio).replace('.', '')}`
}

function shadesOf(scheme: Scheme): Record<string, string> {
  return Object.fromEntries(SHADES.flatMap(([colors, grounds, ratios]) =>
    colors.flatMap(hue => grounds.flatMap(ground => ratios.map(ratio => [
      shadeName(hue, ground, ratio),
      shadeReading(groundsOf(ground, scheme), hexOf(hue, scheme), ratio)
    ])))))
}

// Lists the roles whose color depends on the scheme. The link is Ube Deep, or
// Ube Pale in the dark scheme. An italic aside reads in a warm brown-black, or
// a warm parchment in the dark scheme. Inline code sits on VitePress's gray
// code ground turned to Ube's hue at a low chroma and nearly its lightness.
function schemeRoles(scheme: Scheme): Record<string, string> {
  const gray = oklch(VITEPRESS['gray-soft'][scheme])!
  const ube  = oklch(HUES.ube)!
  const code = scheme === 'light'
    ? { c: 0.026, l: gray.l - 0.008 }
    : { c: 0.04, l: gray.l }
  const aside = scheme === 'light'
    ? { c: 0.035, h: 62, l: 0.39 }
    : { c: 0.036, h: 55, l: 0.83 }
  return {
    aside         : formatHex({ mode: 'oklch', ...aside }),
    'code-ground' : formatHex({ mode: 'oklch', h: ube.h, ...code }),
    link          : scheme === 'light' ? PALETTE['ube-deep'] : PALETTE['ube-pale']
  }
}

export const SCHEME_ROLES = perScheme(schemeRoles)

export const SCHEME_SHADES = perScheme(shadesOf)

// Writes each color as a custom property under one selector.
function block(selector: string, colors: Record<string, string>): string {
  const lines = Object.entries(colors).map(([name, value]) => `  ${name}: ${value};`)
  return `${selector} {\n${lines.join('\n')}\n}\n`
}

// Names each color `<prefix><name>`.
function prefixed(prefix: string, colors: Record<string, string>): Record<string, string> {
  return Object.fromEntries(Object.entries(colors).map(([name, value]) => [`${prefix}${name}`, value]))
}

// Writes each family's accents under every element naming the family and
// under a rule's hover card, its hue beside a nib shade reading 4.5:1 and an
// edge shade reading 3.4:1 on each ground a badge sits on.
function familyAccents(): string {
  const shades = (Object.keys(ACCENT_RATIOS) as (keyof typeof ACCENT_RATIOS)[]).flatMap(kind => ['', '-card', '-chip', '-soft'].map(ground => ({ ground, kind })))
  return Object.keys(FAMILIES).map(family => {
    const lines = [
      `  --family-color: var(--color-family-${family});`,
      ...shades.map(({ ground, kind }) =>
        `  --family-${kind}${ground}: var(--prose-shade-${shadeName(family, ground.slice(1) || 'page', ACCENT_RATIOS[kind])});`)
    ]
    return `[data-family="${family}"],\n[data-popup="rule-card"]:has(.rule-card[data-family="${family}"]) {\n${lines.join('\n')}\n}\n`
  }).join('')
}

// Writes the shades, the shiki ground, and each family's accents, none of which
// Tailwind's theme declares.
export function paletteCss(): string {
  return block(':root', { ...prefixed('--prose-shade-', SCHEME_SHADES.light), '--prose-shiki-dark-bg': GROUNDS.code.dark })
    + block('.dark', prefixed('--prose-shade-', SCHEME_SHADES.dark))
    + familyAccents()
}

// Writes what the Tailwind entry takes from the palette, meaning the palette,
// role, family, and section colors as a `@theme static` block, so Tailwind emits
// each `--color-*` property for the sheets and the utilities alike, beside each
// role's dark value in the layer those properties land in.
export function themeCss(): string {
  const theme = block('@theme static', {
    ...prefixed('--color-', { ...PALETTE, ...SCHEME_ROLES.light }),
    ...prefixed('--color-family-', FAMILIES),
    ...prefixed('--color-section-', SECTIONS)
  })
  return `${theme}@layer theme {\n${block('.dark', prefixed('--color-', SCHEME_ROLES.dark))}}\n`
}
