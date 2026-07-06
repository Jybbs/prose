import path from 'node:path'

import type { OgPage }           from '../../lib/og/pages'
import { loadBrandAssets }       from '../../lib/og/render/assets'
import { landingSvg }            from '../../lib/og/render/landing'
import { fitTitleSize, pageSvg } from '../../lib/og/render/template'

const brand = loadBrandAssets(path.resolve(import.meta.dirname, '../../..'))

const CARD_MODULES = import.meta.glob<{ page: OgPage }>('./cards/*/page.ts', { eager: true })

const CASES: ReadonlyArray<readonly [string, OgPage]> = Object.entries(CARD_MODULES)
  .map(([file, mod]) => [path.basename(path.dirname(file)), mod.page] as const)

describe('pageSvg', () => {
  it('discovers every card case directory', () => {
    expect(CASES.length).toBeGreaterThan(0)
  })

  it.each(CASES)('renders the %s card', async (name, page) => {
    await expect(await pageSvg(page, brand, '0.7.0'))
      .toMatchFileSnapshot(`cards/${name}/output.svg.snap`)
  })
})

describe('landingSvg', () => {
  it('renders the landing card', async () => {
    await expect(await landingSvg(brand, '0.7.0'))
      .toMatchFileSnapshot('cards/landing/output.svg.snap')
  })
})

describe('fitTitleSize', () => {
  it.each([
    [4,  false, 144],
    [5,  false, 132],
    [8,  false, 132],
    [9,  false, 120],
    [14, false, 120],
    [15, false, 100],
    [12, true,  108],
    [13, true,  100],
    [17, true,  100],
    [18, true,  84],
    [22, true,  84],
    [23, true,  76]
  ])('sizes a %i-char title with caption=%s at %i', (length, hasCaption, expected) => {
    expect(fitTitleSize('x'.repeat(length), hasCaption)).toBe(expected)
  })
})
