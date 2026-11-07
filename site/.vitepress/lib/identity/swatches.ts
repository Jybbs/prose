import { converter, wcagContrast } from 'culori'
import { parse }                   from 'postcss'
import type { Rule }               from 'postcss'

import * as palette    from '../shared/palette'
import { toTitleCase } from '../shared/title-case'
import { LEVELS }      from './roles'

import type { Scheme }   from '../shared/palette'
import type { Contrast } from './roles'

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
  hex      : string
  mix     ?: (typeof palette.BRAND_SHADES)[keyof typeof palette.BRAND_SHADES]
  name     : string
  oklch    : string
  title    : string
  token    : string
  uses     : readonly Use[]
}

export interface SourceFile {
  file : string
  text : string
}

// Lists each group of swatches in the order the band sets them out.
export const GROUPS = ['brand', 'complements', 'states', 'sections', 'neutrals'] as const

export type GroupId = (typeof GROUPS)[number]

const ALTERNATION = Object.keys(palette.PALETTE).toSorted((a, b) => b.length - a.length).join('|')

const UTILITIES = 'accent|bg|border|caret|decoration|fill|from|outline|ring|stroke|text|to|via'

const oklch = converter('oklch')

// Returns the property the `--callout-mark` of each callout kind in `sheet`
// reads, a kind with no rule of its own taking the one the rule naming every
// kind reads.
export function calloutMarks(sheet: string): Map<string, string> {
  const marks = new Map<string, string>()
  parse(sheet).walkDecls('--callout-mark', decl => {
    const { selector } = decl.parent as Rule
    const kinds        = /:is\(([^)]*)\)$/.exec(selector)?.[1]!.split(',') ?? [/\.\w+$/.exec(selector)![0]]
    const read         = /var\((--[\w-]+)\)/.exec(decl.value)![1]!
    for (const kind of kinds) marks.set(kind.trim().slice(1), read)
  })
  return marks
}

// Names the callout kind whose mark each hue's shade inks.
export function calloutStates(marks: ReadonlyMap<string, string>): Map<string, string> {
  const shade = new RegExp(`--prose-shade-(${ALTERNATION})-`)
  return new Map([...marks].flatMap(([kind, value]) => {
    const hue = shade.exec(value)?.[1]
    return hue ? [[hue, kind] as const] : []
  }))
}

export function contrastOn(color: string, ground: string): Contrast {
  const ratio = wcagContrast(color, ground)
  return { level: LEVELS.find(([, floor]) => ratio >= floor)?.[0] ?? 'fails', ratio }
}

// Writes a color's OKLCH lightness and chroma to three places and its hue to a
// tenth of a degree.
export function oklchText(color: string): string {
  const { c, h = 0, l } = oklch(color)!
  return `${l.toFixed(3)} ${c.toFixed(3)} ${h.toFixed(1)}°`
}

// Names the palette color a hex is, preferring a hue over a brand shade.
const nameOf = (hex: string): string | undefined =>
  Object.entries(palette.PALETTE).find(([, value]) => value === hex)?.[0]

// Returns the group of the declaration that assigns each palette color, the
// brand hue and its shades first, then the hues a callout's state inks from,
// the hues `FAMILIES` assigns, the hues `SECTIONS` and `GROUNDS` assign, and
// last every hue none of them assigns, which joins the complements.
function groupsOf(states: ReadonlyMap<string, string>): Map<string, GroupId> {
  const groups = new Map<string, GroupId>([
    [palette.BRAND, 'brand'], ...Object.keys(palette.BRAND_SHADES).map(name => [name, 'brand'] as const)
  ])
  const assign = (group: GroupId, hexes: readonly string[]) => {
    for (const name of hexes.map(nameOf)) if (name) groups.getOrInsert(name, group)
  }
  assign('states', [...states.keys()].map(hue => palette.PALETTE[hue as keyof typeof palette.PALETTE]))
  assign('complements', Object.values(palette.FAMILIES))
  assign('sections', Object.values(palette.SECTIONS))
  assign('neutrals', Object.values(palette.GROUNDS).flatMap(Object.values))
  assign('complements', Object.values(palette.HUES))
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
// meaning a declaration reading its `--color-*` property or one of its
// `--prose-shade-*` shades, a Tailwind class naming it, and an expression
// reading it off `PALETTE`.
export function scanUses(sheets: readonly SourceFile[], sources: readonly SourceFile[]): Map<string, Use[]> {
  const uses     = new Map<string, Use[]>()
  const add      = (name: string, use: Use) => { uses.getOrInsert(name, []).push(use) }
  const property = new RegExp(`var\\(--(?:color-(${ALTERNATION})\\)|prose-shade-(${ALTERNATION})-)`, 'g')
  const utility  = new RegExp(`(?<![\\w-])(dark:)?(?:${UTILITIES})-(${ALTERNATION})(?![\\w-])`, 'g')
  const read     = new RegExp(`PALETTE(?:\\.|\\[')(${ALTERNATION})(?![\\w-])`, 'g')

  for (const { file, text } of sheets) {
    parse(text).walkDecls(decl => {
      const selector = decl.parent?.type === 'rule' ? (decl.parent as Rule).selector : ''
      const schemes  = /\.dark\b/.test(selector) ? ['dark'] as const : palette.SCHEMES
      for (const [, whole, shade] of decl.value.matchAll(property)) {
        add((whole ?? shade)!, { schemes, token: decl.prop, where: `${file} ${selector}`.trim() })
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
// group that assigns it, the brand hue first and its shades running from the
// darkest to the lightest, and measures each against the page ground of each
// scheme.
export function swatches(
  scanned : ReadonlyMap<string, readonly Use[]>,
  page    : Record<Scheme, string>,
  states  : ReadonlyMap<string, string>
): Swatch[] {
  const declared = declaredUses()
  const toward   = (name: string) => {
    const mix = palette.BRAND_SHADES[name as keyof typeof palette.BRAND_SHADES]
    return mix ? (mix.toward === 'black' ? -mix.share : mix.share) : -Infinity
  }
  return [...groupsOf(states)]
    .toSorted(([a, x], [b, y]) => GROUPS.indexOf(x) - GROUPS.indexOf(y) || (x === 'brand' ? toward(a) - toward(b) : 0))
    .map(([name, group]) => {
      const hex = palette.PALETTE[name as keyof typeof palette.PALETTE]
      return {
        contrast : palette.perScheme(scheme => contrastOn(hex, page[scheme])),
        group,
        hex,
        mix      : palette.BRAND_SHADES[name as keyof typeof palette.BRAND_SHADES],
        name,
        oklch    : oklchText(hex),
        title    : toTitleCase(name, '-'),
        token    : `--color-${name}`,
        uses     : distinct([...declared.get(name) ?? [], ...scanned.get(name) ?? []])
      }
    })
}
