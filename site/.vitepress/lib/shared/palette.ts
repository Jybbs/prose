import { formatHex, interpolate } from 'culori'

import type { GlossaryFamily, SectionSlug } from './registries'

// The same blend CSS performs for `color-mix(in oklch, base, toward pct%)`
const oklchMix = (base: string, toward: 'black' | 'white', share: number): string =>
  formatHex(interpolate([base, toward], 'oklch')(share / 100))

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

const ROLES = {
  accent       : PALETTE.chambray,
  error        : PALETTE.apricot,
  'link-hover' : PALETTE['ube-deep'],
  warning      : PALETTE.eureka
}

// Keyed by section slug so a rename in the `SECTIONS` registry fails here,
// where the accent tokens the OG cards resolve are emitted. Rules pages
// accent by family instead, so the `rules` slug carries no section hue.
export const SECTIONS: Record<Exclude<SectionSlug, 'rules'>, string> = {
  integrations : PALETTE.rainee,
  primitives   : PALETTE.dexter,
  reference    : PALETTE.casper,
  usage        : PALETTE.oat
}

const GROUPS = {
  family  : FAMILIES,
  palette : PALETTE,
  role    : ROLES,
  section : SECTIONS
}

export function paletteCss(): string {
  const lines = Object.entries(GROUPS).flatMap(([prefix, entries]) =>
    Object.entries(entries).map(([name, value]) => `  --prose-${prefix}-${name}: ${value};`))
  return `:root {\n${lines.join('\n')}\n}\n`
}
