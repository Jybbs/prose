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

import { paletteCss } from '../lib/shared/palette'
import { siteDir }    from '../lib/shared/paths'

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

export const SITE            = siteDir(import.meta.url)
export const TOKENS          = '.vitepress/theme/styles/tokens.css'
export const VITEPRESS_THEME = 'node_modules/vitepress/dist/client/theme-default'

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
  static read(declaring: Root[], reading: Root[]): Vocabulary {
    const vocabulary = new Vocabulary(facedBy(reading), new Set(), mixedBy(reading), new Set())
    for (const root of declaring) {
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

  // Returns this vocabulary with the custom properties the `roots` pass to
  // `color-mix()` or read as a font family in place of its own.
  reading(roots: Root[]): Vocabulary {
    return new Vocabulary(facedBy(roots), this.media, mixedBy(roots), this.properties)
  }

  // Lists each value a stylesheet writes in place of a token, by the line it
  // sits on.
  strays(root: Root): Stray[] {
    const found = this.mediaStrays(root)
    root.walkDecls(decl => { found.push(...this.declarationStrays(decl)) })
    return found.toSorted((a, b) => a.line - b.line)
  }
}

export const read = (file: string): string => fs.readFileSync(path.join(SITE, file), 'utf8')

// Names each custom property the sheets read as a font family, directly or
// through another custom property that a family list reads.
function facedBy(roots: Root[]): Set<string> {
  const faced = new Set<string>()
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
function mixedBy(roots: Root[]): Set<string> {
  const mixed = new Set<string>()
  for (const root of roots) {
    root.walkDecls(decl => {
      walk(parseListOfComponentValues(tokenize({ css: decl.value })), ({ node, parent }) => {
        const ref = named(node, 'var') && named(parent, 'color-mix') ? referenced(node) : undefined
        if (ref) mixed.add(ref[4].value)
      })
    })
  }
  return mixed
}

// Builds the vocabulary the site's stylesheets read from, meaning what
// `tokens.css`, the palette sheet, the VitePress default theme, and the page
// head the config writes declare, beside the custom properties the `reading`
// sheets pass to `color-mix()` or read as a font family.
export function siteVocabulary(tokens: Root, reading: Root[]): Vocabulary {
  const vocabulary = Vocabulary.read([tokens, parse(paletteCss()), ...vitepressSheets()], reading)
  for (const [, prop] of read('.vitepress/config.ts').matchAll(/(--prose-[\w-]+):/g)) {
    vocabulary.properties.add(prop)
  }
  return vocabulary
}

// Parses each stylesheet the VitePress default theme ships.
export function vitepressSheets(): Root[] {
  return fs.globSync(`${VITEPRESS_THEME}/styles/**/*.css`, { cwd: SITE }).map(file => parse(read(file)))
}

function named(node: unknown, name: string): node is FunctionNode {
  return isFunctionNode(node) && node.getName().toLowerCase() === name
}

// Returns the ident naming the custom property a `var()` reads.
function referenced(node: FunctionNode): TokenIdent | undefined {
  const token = node.value.find(isTokenNode)?.value
  return isTokenIdent(token) ? token : undefined
}
