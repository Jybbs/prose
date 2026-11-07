import fs from 'node:fs'

import { parse } from 'postcss'

import { read, SITE, TOKENS } from '../vocabulary'

const GENERIC_FAMILIES = new Set(['monospace', 'serif', 'ui-monospace'])

// Lists each face a font stack in the token sheet names, without the
// `Variable` suffix of a variable build, the generic families aside.
const FACES = new Set<string>()
parse(read(TOKENS)).walkDecls(/font/, ({ value }) => {
  if (!value.includes(',')) return
  for (const face of value.split(',').map(each => each.trim().replaceAll(/^["']|["']$/g, '').replace(/ Variable$/, ''))) {
    if (!GENERIC_FAMILIES.has(face)) FACES.add(face)
  }
})

// Matches a hex color, a color function over literal channels, a length with
// its unit, and a face the token sheet declares, each a value a specimen reads
// from its declaration.
const RESTATED = new RegExp(String.raw`(?<![\w$&{-])#[\da-f]{3,8}\b|\b(?:rgba?|hsla?|oklch|oklab|lab|lch)\(\s*[\d.]|(?<![\w$.-])\d+(?:\.\d+)?(?:px|rem|em|ch|vw|vh)\b|\b(?:${[...FACES].join('|')})\b`, 'gi')

const SPECIMENS = fs.globSync(['.vitepress/lib/identity/*.ts', '.vitepress/theme/components/identity/**/*.{css,tsx}'], { cwd: SITE })

describe('the visual identity specimens', () => {
  it('cover the modules and components the page draws from', () => {
    expect(SPECIMENS.length).toBeGreaterThan(0)
  })

  it.each(SPECIMENS)('%s restates no value a declaration holds', file => {
    const text = fs.readFileSync(`${SITE}/${file}`, 'utf8')
    expect([...text.matchAll(RESTATED)].map(([value]) => value)).toStrictEqual([])
  })
})
