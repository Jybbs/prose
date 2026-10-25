import fs   from 'node:fs'
import path from 'node:path'

import {
  isFunctionNode,
  isTokenNode,
  isWhitespaceNode,
  parseListOfComponentValues,
  walk
} from '@csstools/css-parser-algorithms'
import {
  isTokenComma,
  isTokenDimension,
  isTokenHash,
  isTokenIdent,
  isTokenNumeric,
  isTokenPercentage,
  isTokenString,
  tokenize
} from '@csstools/css-tokenizer'
import { parse as parseColor } from 'culori'
import { parse }               from 'postcss'

import { paletteCss } from '../../lib/shared/palette'
import { siteDir }    from '../../lib/shared/paths'

import type { ComponentValue, FunctionNode } from '@csstools/css-parser-algorithms'
import type { TokenIdent }                   from '@csstools/css-tokenizer'
import type { Declaration, Root }            from 'postcss'

// Holds the absolute and font-relative length units CSS Values 4 defines,
// leaving out the viewport and container units, which measure a share of the
// viewport or of a container.
const LENGTH_UNITS   = new Set([
  'cap', 'ch', 'cm', 'em', 'ex', 'ic', 'in', 'lh', 'mm', 'pc', 'pt', 'px', 'q', 'rcap', 'rch',
  'rem', 'rex', 'ric', 'rlh'
])
// Holds the keywords the `font` shorthand writes for its size or line height,
// meaning the `font-size` keywords CSS Fonts 4 defines and the `normal` line
// height.
const SIZE_KEYWORDS  = new Set([
  'large', 'larger', 'math', 'medium', 'normal', 'small', 'smaller', 'x-large', 'x-small',
  'xx-large', 'xx-small', 'xxx-large'
])
const COLOR_KEYWORDS = new Set(['currentcolor', 'transparent'])
const SYSTEM_FONTS   = new Set([
  'caption', 'icon', 'menu', 'message-box', 'small-caption', 'status-bar'
])
const WIDE_KEYWORDS  = new Set(['inherit', 'initial', 'revert', 'revert-layer', 'unset'])

const SITE   = siteDir(import.meta.url)
const TOKENS = '.vitepress/theme/styles/tokens.css'

type Kind = 'color' | 'length' | 'media' | 'share' | 'token' | 'typeface'

interface Stray {
  kind  : Kind
  line  : number
  value : string
}

// Holds the custom media and properties the token sheets declare, beside the
// custom properties the stylesheets pass to `color-mix()` or read as a font
// family.
class Vocabulary {
  constructor(
    readonly faces      : Set<string>,
    readonly media      : Set<string>,
    readonly mixed      : Set<string>,
    readonly properties : Set<string>
  ) {}

  // Reads each custom property and custom media the `declaring` sheets declare,
  // and each custom property the `reading` sheets pass to `color-mix()` or read
  // as a font family.
  static read(declaring: string[], reading: string[]): Vocabulary {
    const vocabulary = new Vocabulary(facedBy(reading), new Set(), mixedBy(reading), new Set())
    for (const sheet of declaring) {
      const root = parse(sheet)
      root.walkDecls(/^--/, decl => { vocabulary.properties.add(decl.prop) })
      root.walkAtRules('custom-media', rule => { vocabulary.media.add(rule.params.split(/\s/)[0]) })
    }
    return vocabulary
  }

  // Lists each value a declaration writes in place of a token, meaning a length,
  // a color, a typeface, or a `color-mix()` share, beside each `--prose-*`
  // property it declares and each `--prose-*` or `--vp-*` property it reads that
  // nothing declares.
  private declarationStrays(decl: Declaration): Stray[] {
    const found: Stray[] = []
    if (decl.prop.startsWith('--prose-')) {
      found.push({ kind: 'token', line: decl.source!.start!.line, value: decl.prop })
    }
    const lead    = decl.prop.length + decl.raws.between!.length
    const report  = (kind: Kind, offset: number, value: string) =>
      found.push({ kind, line: decl.positionInside(lead + offset).line, value })
    const nodes   = parseListOfComponentValues(tokenize({ css: decl.value }))
    const parents = new Map<unknown, unknown>()

    walk(nodes, ({ node, parent }) => {
      parents.set(node, parent)
      if (named(node, 'var')) {
        const ref = referenced(node)
        if (ref && /^--(prose|vp)-/.test(ref[4].value) && !this.properties.has(ref[4].value)) {
          report('token', ref[2], ref[4].value)
        }
        return
      }
      if (isFunctionNode(node)) {
        if (parseColor(node.toString())) report('color', node.name[2], node.toString())
        return
      }
      if (!isTokenNode(node)) return
      const token = node.value
      const host  = named(parent, 'var') ? parents.get(parent) : parent
      if (isTokenDimension(token) && LENGTH_UNITS.has(token[4].unit.toLowerCase())) {
        report('length', token[2], token[1])
      } else if (isTokenPercentage(token)
        && (named(host, 'color-mix') || (parent.value === nodes && this.mixed.has(decl.prop)))) {
        report('share', token[2], token[1])
      } else if ((isTokenHash(token) || isTokenIdent(token))
        && !COLOR_KEYWORDS.has(token[1].toLowerCase()) && parseColor(token[1])) {
        report('color', token[2], token[1])
      }
    })

    const face = decl.prop === 'font-family' || this.faces.has(decl.prop)
    if (face || decl.prop === 'font') {
      for (const { value: token } of families(!face, nodes).filter(isTokenNode)) {
        const family = isTokenIdent(token) && !WIDE_KEYWORDS.has(token[4].value.toLowerCase())
        if (isTokenString(token) || family) report('typeface', token[2], token[1])
      }
    }
    return found
  }

  // Lists each media or container query that writes a dimension, and each media
  // query naming a custom media nothing declares, beside each custom media a
  // stylesheet declares for itself.
  private mediaStrays(root: Root): Stray[] {
    const found: Stray[] = []
    root.walkAtRules('custom-media', rule => {
      found.push({ kind: 'media', line: rule.source!.start!.line, value: rule.params.split(/\s/)[0] })
    })
    root.walkAtRules(/^(container|media)$/, rule => {
      for (const token of tokenize({ css: rule.params })) {
        const custom = rule.name === 'media'
          && isTokenIdent(token) && token[4].value.startsWith('--')
        if (isTokenDimension(token) || (custom && !this.media.has(token[4].value))) {
          found.push({ kind: 'media', line: rule.source!.start!.line, value: token[1] })
        }
      }
    })
    return found
  }

  // Lists each value a stylesheet writes in place of a token, by the line it
  // sits on.
  strays(css: string): Stray[] {
    const root  = parse(css)
    const found = this.mediaStrays(root)
    root.walkDecls(decl => { found.push(...this.declarationStrays(decl)) })
    return found.toSorted((a, b) => a.line - b.line)
  }
}

const read = (file: string): string => fs.readFileSync(path.join(SITE, file), 'utf8')

const STYLESHEETS = fs.globSync(['.vitepress/theme/**/*.css', 'prototypes/**/*.css'], { cwd: SITE })
  .filter(file => file !== TOKENS)
  .toSorted()

const TOKEN_SHEET = read(TOKENS)
const VITEPRESS   = fs.globSync('node_modules/vitepress/dist/client/theme-default/styles/*.css', { cwd: SITE })
const VOCABULARY  = Vocabulary.read(
  [TOKEN_SHEET, paletteCss(), ...VITEPRESS.map(read)],
  STYLESHEETS.map(read)
)

// Adds the custom properties the config writes into the page head, which carry
// the dark code-block ground.
for (const [, prop] of read('.vitepress/config.ts').matchAll(/(--prose-[\w-]+):/g)) {
  VOCABULARY.properties.add(prop)
}

// Names each custom property the sheets read as a font family, directly or
// through another custom property that a family list reads.
function facedBy(sheets: string[]): Set<string> {
  const faced = new Set<string>()
  const roots = sheets.map(sheet => parse(sheet))
  for (let size = -1; size !== faced.size;) {
    size = faced.size
    for (const root of roots) {
      root.walkDecls(decl => {
        if (decl.prop !== 'font' && decl.prop !== 'font-family' && !faced.has(decl.prop)) return
        const nodes = parseListOfComponentValues(tokenize({ css: decl.value }))
        for (const node of families(decl.prop === 'font', nodes)) {
          const ref = named(node, 'var') ? referenced(node) : undefined
          if (ref) faced.add(ref[4].value)
        }
      })
    }
  }
  return faced
}

// Returns the family list a `font` or `font-family` value writes, beside the
// fallback each `var()` in it holds. In the `font` shorthand the list starts
// after the last size or line height ahead of its first comma, or at the last
// value there where a `var()` holds the size, and a system font keyword holds
// no family.
function families(shorthand: boolean, nodes: ComponentValue[]): ComponentValue[] {
  const keyword = (node: ComponentValue) =>
    isTokenNode(node) && isTokenIdent(node.value) ? node.value[4].value.toLowerCase() : ''
  const comma = nodes.findIndex(node => isTokenNode(node) && isTokenComma(node.value))
  const head  = nodes.slice(0, comma < 0 ? nodes.length : comma)
  const last  = head.findLastIndex(node => !isWhitespaceNode(node))
  const sized = head.findLastIndex(node => isFunctionNode(node) || SIZE_KEYWORDS.has(keyword(node))
    || (isTokenNode(node) && isTokenNumeric(node.value)))
  if (shorthand && comma < 0 && last === 0 && SYSTEM_FONTS.has(keyword(head[0]))) return []
  return nodes.slice(shorthand ? Math.min(sized + 1, last) : 0).flatMap(node => {
    if (!named(node, 'var')) return [node]
    const fallback = node.value.findIndex(item => isTokenNode(item) && isTokenComma(item.value))
    return fallback < 0 ? [node] : [node, ...families(false, node.value.slice(fallback + 1))]
  })
}

// Names each custom property the sheets read as a `color-mix()` argument,
// since a percentage such a property holds is a share.
function mixedBy(sheets: string[]): Set<string> {
  const mixed = new Set<string>()
  for (const sheet of sheets) {
    parse(sheet).walkDecls(decl => {
      walk(parseListOfComponentValues(tokenize({ css: decl.value })), ({ node, parent }) => {
        const ref = named(node, 'var') && named(parent, 'color-mix') ? referenced(node) : undefined
        if (ref) mixed.add(ref[4].value)
      })
    })
  }
  return mixed
}

function named(node: unknown, name: string): node is FunctionNode {
  return isFunctionNode(node) && node.getName().toLowerCase() === name
}

// Returns the ident naming the custom property a `var()` reads.
function referenced(node: FunctionNode): TokenIdent | undefined {
  const token = node.value.find(isTokenNode)?.value
  return isTokenIdent(token) ? token : undefined
}

describe('the site stylesheets', () => {
  it.each(STYLESHEETS)('%s writes every value through a declared token', file => {
    expect(VOCABULARY.strays(read(file))).toStrictEqual([])
  })
})

describe('tokens.css', () => {
  const root   = parse(TOKEN_SHEET)
  const tokens = [] as Declaration[]
  root.walkDecls(/^--prose-/, decl => { tokens.push(decl) })

  it('declares each literal once within a family', () => {
    const literals = tokens.filter(({ value }) => !value.includes('var('))
    const groups   = Map.groupBy(literals, ({ prop, value }) => `${prop.split('-')[3]} ${value}`)
    const repeated = [...groups.values()].filter(group => group.length > 1)
    expect(repeated.map(group => group.map(({ prop }) => prop))).toStrictEqual([])
  })

  it('reads only tokens something declares', () => {
    const unresolved = VOCABULARY.strays(TOKEN_SHEET)
      .filter(({ kind, value }) => kind === 'token' && !VOCABULARY.properties.has(value))
    expect(unresolved).toStrictEqual([])
  })

  it('declares no token the site leaves unread', () => {
    const sources = fs.globSync(['.vitepress/{lib,theme}/**/*.{css,ts,vue}', 'prototypes/**/*.{css,ts,vue}'], { cwd: SITE })
      .filter(file => file !== TOKENS)
      .map(read)
    root.walkDecls(({ value }) => { sources.push(value) })
    const readers = sources.join('\n')
    const unread  = tokens.filter(({ prop }) => !new RegExp(`${prop}(?![\\w-])`).test(readers))
    expect(unread.map(({ prop }) => prop)).toStrictEqual([])
  })
})

describe('strays', () => {
  const strayed = (css: string) =>
    new Vocabulary(facedBy([css]), VOCABULARY.media, mixedBy([css]), VOCABULARY.properties).strays(css)

  it.each([
    { css: '.a { margin: 0 14px; }',                                                            kind: 'length',   value: '14px' },
    { css: '.a { width: var(--a-width, 30rem); }',                                              kind: 'length',   value: '30rem' },
    { css: '.a { gap: 0.5em; }',                                                                kind: 'length',   value: '0.5em' },
    { css: '.a { color: color-mix(in srgb, var(--vp-c-bg) 55%, transparent); }',                kind: 'share',    value: '55%' },
    { css: '.a { color: color-mix(in srgb, var(--vp-c-bg) var(--a, 6%), transparent); }',       kind: 'share',    value: '6%' },
    { css: '.a { --a: 12%; color: color-mix(in srgb, var(--vp-c-bg) var(--a), transparent); }', kind: 'share',    value: '12%' },
    { css: '.a { color: #8a80cb; }',                                                            kind: 'color',    value: '#8a80cb' },
    { css: '.a { border-color: rebeccapurple; }',                                               kind: 'color',    value: 'rebeccapurple' },
    { css: '.a { color: rgb(0 0 0 / 28%); }',                                                   kind: 'color',    value: 'rgb(0 0 0 / 28%)' },
    { css: '.a { font-family: Georgia; }',                                                      kind: 'typeface', value: 'Georgia' },
    { css: '.a { font: var(--prose-text-1rem) "Lora"; }',                                       kind: 'typeface', value: '"Lora"' },
    { css: '.a { font: 600 var(--prose-text-1rem) Georgia; }',                                  kind: 'typeface', value: 'Georgia' },
    { css: '.a { --a: Georgia; font-family: var(--a); }',                                       kind: 'typeface', value: 'Georgia' },
    { css: '.a { font: italic 100% Georgia; }',                                                 kind: 'typeface', value: 'Georgia' },
    { css: '.a { font: italic var(--prose-text-1rem) Georgia; }',                               kind: 'typeface', value: 'Georgia' },
    { css: '.a { font: italic bold larger Georgia; }',                                          kind: 'typeface', value: 'Georgia' },
    { css: '.a { font: var(--prose-text-1rem) / normal Georgia; }',                             kind: 'typeface', value: 'Georgia' },
    { css: '.a { font: var(--prose-text-1rem) Georgia, var(--vp-font-family-base); }',          kind: 'typeface', value: 'Georgia' },
    { css: '.a { font-family: Georgia, var(--vp-font-family-base); }',                          kind: 'typeface', value: 'Georgia' },
    { css: '.a { font-family: var(--vp-font-family-base, Georgia); }',                          kind: 'typeface', value: 'Georgia' },
    { css: '.a { --a: Georgia; font: var(--prose-text-1rem) var(--a); }',                       kind: 'typeface', value: 'Georgia' },
    { css: '.a { --a: var(--b); --b: Georgia; font-family: var(--a); }',                        kind: 'typeface', value: 'Georgia' },
    { css: '@container (min-width: 400px) { .a { color: inherit; } }',                          kind: 'media',    value: '400px' },
    { css: '@custom-media --a (max-width: 999px);',                                             kind: 'media',    value: '--a' },
    { css: '@media (min-width: 1280px) { .a { color: inherit; } }',                             kind: 'media',    value: '1280px' },
    { css: '@media (--prose-bp-huge) { .a { color: inherit; } }',                               kind: 'media',    value: '--prose-bp-huge' },
    { css: '.a { gap: var(--prose-space-19); }',                                                kind: 'token',    value: '--prose-space-19' },
    { css: '.a { --prose-panel-wash: var(--prose-tint-30); }',                                  kind: 'token',    value: '--prose-panel-wash' },
    { css: '.a { color: var(--vp-c-txt-1); }',                                                  kind: 'token',    value: '--vp-c-txt-1' }
  ])('reports the $kind $value', ({ css, kind, value }) => {
    expect(strayed(css)).toStrictEqual([{ kind, line: 1, value }])
  })

  it('reports each stray on the line it sits on inside a value spanning lines', () => {
    const css = '.a {\n  transition :\n    gap 0.2s,\n    margin 0.2s;\n  box-shadow :\n    0 0 0\n    2px;\n}'
    expect(strayed(css)).toStrictEqual([{ kind: 'length', line: 7, value: '2px' }])
  })

  it('passes values read through the tokens and the units no token measures', () => {
    const css = [
      '.a {',
      '  --a-offset  : 60%;',
      '  margin      : calc(var(--prose-space-14) * -1) var(--prose-space-1-5rem);',
      '  color       : color-mix(in srgb, var(--vp-c-text-1) var(--prose-tint-55), transparent);',
      '  border      : var(--prose-border-1) solid currentColor;',
      '  width       : calc(100vw - var(--prose-space-32));',
      '  transform   : translate(-50%, var(--a-offset)) rotate(-6deg);',
      '  font-family : var(--vp-font-family-mono);',
      '  font        : italic 600 var(--prose-text-sm) / 1.4 var(--prose-font-display);',
      '  line-height : 1.6;',
      '}',
      '.b { font-family: inherit; }',
      '.c { --weight: bold; font: var(--weight) var(--prose-text-sm) var(--prose-font-display); }',
      '.d { font: caption; }',
      '@container card (inline-size > 0) { .a { color: inherit; } }',
      '@container --card (inline-size > 0) { .a { color: inherit; } }',
      '@media (--prose-bp-wide) { .a { color: inherit; } }',
      '@media (prefers-reduced-motion: reduce) { .a { color: inherit; } }'
    ].join('\n')
    expect(strayed(css)).toStrictEqual([])
  })
})
