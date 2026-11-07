import fs from 'node:fs'

import { isTokenNode, isWhitespaceNode, parseListOfComponentValues } from '@csstools/css-parser-algorithms'
import { isTokenDelim, isTokenIdent, tokenize }                      from '@csstools/css-tokenizer'
import { parse }                                                     from 'postcss'

import { siteSheets }  from '../support'
import * as tailwind   from '../tailwind'
import * as vocabulary from '../vocabulary'

import type { Declaration, Root, Rule } from 'postcss'

const SHEETS      = siteSheets()
const STYLESHEETS = SHEETS.filter(({ file }) => file !== vocabulary.TOKENS)
const TOKEN_SHEET = SHEETS.find(({ file }) => file === vocabulary.TOKENS)!.root
const VOCABULARY  = vocabulary.siteVocabulary(TOKEN_SHEET, STYLESHEETS.map(({ root }) => root))
// Holds the CSS Tailwind compiles for each class the site writes, which the
// unread-token check reads in place of the `@theme` blocks, since a block maps
// a token to a utility whether or not any class reads it.
const GENERATED   = tailwind.generatedCss(
  await tailwind.designSystem(),
  STYLESHEETS.map(({ root }) => root)
)

// Returns the element each compound of `selector` opens on, from the outermost
// to the subject, writing `*` for a compound that opens on a class, an
// attribute, or a pseudo-class. A function such as `:is()` parses as one
// value, so its commas and spaces start no compound.
function elements(selector: string): string[] {
  const names = [] as string[]
  let opens   = true
  for (const node of parseListOfComponentValues(tokenize({ css: selector }))) {
    const token = isTokenNode(node) ? node.value : undefined
    if (isWhitespaceNode(node) || (isTokenDelim(token) && '>+~'.includes(token[4].value))) {
      opens = true
    } else if (opens) {
      names.push(isTokenIdent(token) ? token[4].value.toLowerCase() : '*')
      opens = false
    }
  }
  return names
}

// Lists each selector that sets an `em`, `i`, `strong`, `b`, or `mark` outside a
// link in a color other than a text color or the aside ink, by the line its
// declaration sits on.
function inlineHues(root: Root): { line: number, selector: string }[] {
  const found = [] as { line: number, selector: string }[]
  root.walkDecls('color', decl => {
    if (vocabulary.TEXT_COLOR.test(decl.value) || decl.parent?.type !== 'rule') return
    for (const selector of (decl.parent as Rule).selectors) {
      const names = elements(selector)
      if (vocabulary.INLINE_TEXT.has(names.at(-1)!) && !names.includes('a')) {
        found.push({ line: decl.source!.start!.line, selector })
      }
    }
  })
  return found
}

describe('the site stylesheets', () => {
  it.each(STYLESHEETS)('$file writes every value through a declared token', ({ root }) => {
    expect(VOCABULARY.strays(root)).toStrictEqual([])
  })

  it('declares each custom property a script reads off a computed style', () => {
    const declared = new Set(VOCABULARY.properties)
    for (const { root } of STYLESHEETS) root.walkDecls(/^--/, ({ prop }) => { declared.add(prop) })
    const read = fs.globSync('.vitepress/{kit,lib,theme}/**/*.{ts,tsx}', { cwd: vocabulary.SITE })
      .flatMap(file => [...vocabulary.read(file).matchAll(/getPropertyValue\('(--[\w-]+)'\)/g)].map(([, name]) => name))
    expect(read.length).toBeGreaterThan(0)
    expect(read.filter(name => !declared.has(name))).toStrictEqual([])
  })

  it.each(STYLESHEETS)(
    '$file sets each em, i, strong, b, and mark outside a link in a text color or the aside ink',
    ({ root }) => {
      expect(inlineHues(root)).toStrictEqual([])
    }
  )

  it('generates no utility setting an em, i, strong, b, or mark outside a link in a hue', () => {
    expect(inlineHues(parse(GENERATED))).toStrictEqual([])
  })
})

describe('inlineHues', () => {
  it.each([
    '.landing-cta-lede em',
    '.landing-section h2 strong',
    '.pq-row > p > b',
    ':is(.tip, .info) mark',
    '.card i',
    '.card>em',
    '.card EM'
  ])('reports %s set in a hue', selector => {
    expect(inlineHues(parse(`${selector} { color: var(--vp-c-brand-1); }`)))
      .toStrictEqual([{ line: 1, selector }])
  })

  it('passes a text color, the aside ink, and an inline element inside a link', () => {
    const css = [
      '.rule-card-caption em { color: var(--vp-c-text-1); }',
      '.vp-doc p em { color: var(--color-aside); }',
      '.landing-section p em { color: inherit; }',
      '.pq-row > p a strong { color: var(--color-link); }',
      '.vp-doc a.body-link em { color: var(--color-link); }'
    ].join('\n')
    expect(inlineHues(parse(css))).toStrictEqual([])
  })
})

describe('tokens.css', () => {
  const tokens = [] as Declaration[]
  TOKEN_SHEET.walkDecls(/^--prose-/, decl => { tokens.push(decl) })

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
    const sources = fs.globSync(['.vitepress/{kit,lib,theme}/**/*.{ts,tsx}', 'prototypes/**/*.{ts,tsx}'], { cwd: vocabulary.SITE })
      .map(vocabulary.read)
    for (const { root } of STYLESHEETS) {
      const sheet = root.clone()
      sheet.walkAtRules('theme', rule => { rule.remove() })
      sources.push(sheet.toString())
    }
    TOKEN_SHEET.walkDecls(({ value }) => { sources.push(value) })
    const readers = [...sources, GENERATED].join('\n')
    const unread  = tokens.filter(({ prop }) => !new RegExp(`${prop}(?![\\w-])`).test(readers))
    expect(unread.map(({ prop }) => prop)).toStrictEqual([])
  })
})

describe('strays', () => {
  const strayed = (css: string) => {
    const root = parse(css)
    return VOCABULARY.reading([root]).strays(root)
  }

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
    { css: '@media (min-width: 1300px) { .a { color: inherit; } }',                             kind: 'media',    value: '1300px' },
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
      '@theme { --breakpoint-wide: 1280px; }',
      '@media (width >= 1280px) { .a { color: inherit; } }',
      '@media (prefers-reduced-motion: reduce) { .a { color: inherit; } }'
    ].join('\n')
    expect(strayed(css)).toStrictEqual([])
  })
})
