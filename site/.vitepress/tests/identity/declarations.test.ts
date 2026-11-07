import fs from 'node:fs'

import { SITE } from '../vocabulary'

// Matches a hex color, a length with its unit, and a typeface the site
// declares, each a value a specimen reads from its declaration.
const RESTATED = /(?<![\w$&{-])#[\da-f]{3,8}\b|(?<![\w$.-])\d+(?:\.\d+)?(?:px|rem|em|ch|vw|vh)\b|\b(?:Fraunces|Georgia|JetBrains|Lora|Menlo)\b/gi

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
