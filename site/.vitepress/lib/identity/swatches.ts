import { converter, wcagContrast } from 'culori'
import { parse }                   from 'postcss'

import * as palette    from '../shared/palette'
import { toTitleCase } from '../shared/title-case'

import type { Scheme } from '../shared/palette'

export type Level = 'fails' | 'mark' | 'text'

interface Contrast {
  level : Level
  ratio : number
}

// Records one place the site reads a color, meaning the property, the class, or
// the expression that reads it, the file or selector it sits in, and the schemes
// it applies in.
interface Use {
  schemes : readonly Scheme[]
  token   : string
  where   : string
}

export interface Swatch {
  contrast : Record<Scheme, Contrast>
  group    : GroupId
  mix     ?: (typeof palette.BRAND_SHADES)[keyof typeof palette.BRAND_SHADES]
  name     : string
  oklch    : Record<Scheme, string>
  title    : string
  token    : string
  uses     : readonly Use[]
  values   : Record<Scheme, string>
}

export interface SourceFile {
  file : string
  text : string
}

// Lists each group of swatches in the order the band sets them out.
export const GROUPS = ['brand', 'families', 'sections', 'grounds', 'hues', 'roles'] as const

export type GroupId = (typeof GROUPS)[number]

const LEVELS = [['text', 4.5], ['mark', 3]] as const

// Lists the name of every palette color and every scheme role.
export const NAMES = [...Object.keys(palette.PALETTE), ...Object.keys(palette.SCHEME_ROLES.light)]

const UTILITIES = 'accent|bg|border|caret|decoration|fill|from|outline|ring|stroke|text|to|via'

const oklch = converter('oklch')

export function contrastOn(color: string, ground: string): Contrast {
  const ratio = wcagContrast(color, ground)
  return { level: LEVELS.find(([, floor]) => ratio >= floor)?.[0] ?? 'fails', ratio }
}

// Writes a color as `oklch(L% C H)`, its lightness to a tenth of a percent, its
// chroma to three places, and its hue to a tenth of a degree.
export function oklchText(color: string): string {
  const { c, h = 0, l } = oklch(color)!
  return `oklch(${(l * 100).toFixed(1)}% ${c.toFixed(3)} ${h.toFixed(1)})`
}

// Names the palette color a hex is, preferring a hue over a brand shade.
const nameOf = (hex: string): string | undefined =>
  Object.entries(palette.PALETTE).find(([, value]) => value === hex)?.[0]

// Returns the group of the declaration that assigns each palette color, the
// brand hue and its shades first, then the hues `FAMILIES`, `SECTIONS`, and
// `GROUNDS` assign, then every hue none of them assigns.
function groupsOf(): Map<string, GroupId> {
  const groups = new Map<string, GroupId>([
    [palette.BRAND, 'brand'], ...Object.keys(palette.BRAND_SHADES).map(name => [name, 'brand'] as const)
  ])
  const assign = (group: GroupId, hexes: readonly string[]) => {
    for (const name of hexes.map(nameOf)) if (name) groups.getOrInsert(name, group)
  }
  assign('families', Object.values(palette.FAMILIES))
  assign('sections', Object.values(palette.SECTIONS))
  assign('grounds', Object.values(palette.GROUNDS).flatMap(Object.values))
  assign('hues', Object.values(palette.HUES))
  return groups
}

// Lists the uses `palette.ts` declares itself, meaning the family, section,
// and role colors its theme block writes and the grounds it measures against.
function declaredUses(): Map<string, Use[]> {
  const uses = new Map<string, Use[]>()
  const add  = (name: string | undefined, use: Use) => { if (name) uses.getOrInsert(name, []).push(use) }
  const where = 'palette.ts'
  for (const [kind, hexes] of [['family', palette.FAMILIES], ['section', palette.SECTIONS]] as const) {
    for (const [name, hex] of Object.entries(hexes)) {
      add(nameOf(hex), { schemes: palette.SCHEMES, token: `--color-${kind}-${name}`, where })
    }
  }
  for (const [ground, schemes] of Object.entries(palette.GROUNDS)) {
    for (const scheme of palette.SCHEMES) add(nameOf(schemes[scheme]), { schemes: [scheme], token: `GROUNDS.${ground}`, where })
  }
  for (const scheme of palette.SCHEMES) {
    for (const [role, hex] of Object.entries(palette.SCHEME_ROLES[scheme])) {
      add(nameOf(hex), { schemes: [scheme], token: `--color-${role}`, where })
    }
  }
  return uses
}

// Lists each use of a palette color the site's stylesheets and sources make,
// meaning a declaration reading its `--color-*` property, a Tailwind class
// naming it, and an expression reading it off `PALETTE`.
export function scanUses(sheets: readonly SourceFile[], sources: readonly SourceFile[]): Map<string, Use[]> {
  const uses = new Map<string, Use[]>()
  const add  = (name: string, use: Use) => { uses.getOrInsert(name, []).push(use) }
  const alternation = NAMES.toSorted((a, b) => b.length - a.length).join('|')
  const property    = new RegExp(`var\\(--color-(${alternation})\\)`, 'g')
  const utility     = new RegExp(`(?<![\\w-])(dark:)?(?:${UTILITIES})-(${alternation})(?![\\w-])`, 'g')
  const read        = new RegExp(`PALETTE(?:\\.|\\[')(${alternation})(?![\\w-])`, 'g')

  for (const { file, text } of sheets) {
    parse(text).walkDecls(decl => {
      const selector = decl.parent?.type === 'rule' ? (decl.parent as unknown as { selector: string }).selector : ''
      const schemes  = /\.dark\b/.test(selector) ? ['dark'] as const : palette.SCHEMES
      for (const [, name] of decl.value.matchAll(property)) {
        add(name!, { schemes, token: decl.prop, where: `${file} ${selector}`.trim() })
      }
    })
  }
  for (const { file, text } of sources) {
    for (const [token, dark, name] of text.matchAll(utility)) {
      add(name!, { schemes: dark ? ['dark'] : palette.SCHEMES, token, where: file })
    }
    for (const [token, name] of text.matchAll(read)) {
      add(name!, { schemes: palette.SCHEMES, token: token.replace(`['`, '.'), where: file })
    }
  }
  return uses
}

// Collapses the uses that repeat one token in one place.
const distinct = (uses: readonly Use[]): Use[] =>
  [...new Map(uses.map(use => [`${use.token} ${use.where} ${use.schemes.join()}`, use])).values()]

// Builds one swatch for each color the palette declares, in the order of the
// group that assigns it, beside each scheme role, measuring each against the
// page ground of each scheme.
export function swatches(scanned: ReadonlyMap<string, readonly Use[]>, page: Record<Scheme, string>): Swatch[] {
  const groups   = groupsOf()
  const declared = declaredUses()
  const swatch   = (name: string, group: GroupId, values: Record<Scheme, string>): Swatch => ({
    contrast : palette.perScheme(scheme => contrastOn(values[scheme], page[scheme])),
    group,
    mix      : palette.BRAND_SHADES[name as keyof typeof palette.BRAND_SHADES],
    name,
    oklch    : palette.perScheme(scheme => oklchText(values[scheme])),
    title    : toTitleCase(name, '-'),
    token    : `--color-${name}`,
    uses     : distinct([...declared.get(name) ?? [], ...scanned.get(name) ?? []]),
    values
  })
  return [
    ...[...groups].toSorted(([, a], [, b]) => GROUPS.indexOf(a) - GROUPS.indexOf(b))
      .map(([name, group]) => swatch(name, group, palette.perScheme(() => palette.PALETTE[name as keyof typeof palette.PALETTE]))),
    ...Object.keys(palette.SCHEME_ROLES.light).map(role => swatch(role, 'roles', palette.perScheme(scheme => palette.SCHEME_ROLES[scheme][role]!)))
  ]
}
