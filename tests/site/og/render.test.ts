import path from 'node:path'

import type { OgPage }           from '../../../site/.vitepress/lib/og/pages'
import { loadBrandAssets }       from '../../../site/.vitepress/lib/og/render/assets'
import { iconSvg }               from '../../../site/.vitepress/lib/og/render/icon'
import { landingSvg }            from '../../../site/.vitepress/lib/og/render/landing'
import * as parts                from '../../../site/.vitepress/lib/og/render/parts'
import { fitTitleSize, pageSvg } from '../../../site/.vitepress/lib/og/render/template'
import { publicDir }             from '../../../site/.vitepress/lib/shared/paths'

const brand = loadBrandAssets(publicDir(import.meta.url))

const CARD_MODULES = import.meta.glob<{ page: OgPage }>('./cards/*/page.ts', { eager: true })

const CASES: ReadonlyArray<readonly [string, OgPage]> = Object.entries(CARD_MODULES)
  .map(([file, mod]) => [path.basename(path.dirname(file)), mod.page] as const)

describe('pageSvg', () => {
  it('discovers every card case directory', () => {
    expect(CASES.length).toBeGreaterThan(0)
  })

  it.each(CASES)('renders the %s card', async (name, page) => {
    await expect(pageSvg(page, brand, '0.7.0')).resolves
      .toMatchFileSnapshot(`cards/${name}/output.svg.snap`)
  })
})

describe('iconSvg', () => {
  it('renders the touch icon', async () => {
    await expect(iconSvg(brand)).resolves.toMatchFileSnapshot('cards/icon/output.svg.snap')
  })
})

describe('landingSvg', () => {
  it('renders the landing card', async () => {
    await expect(landingSvg(brand, '0.7.0')).resolves
      .toMatchFileSnapshot('cards/landing/output.svg.snap')
  })
})

describe('cards with no version', () => {
  it('renders a landing card shorter than the versioned one', async () => {
    const [bare, versioned] = await Promise.all([landingSvg(brand, null), landingSvg(brand, '0.7.0')])
    expect(bare.length).toBeLessThan(versioned.length)
  })

  it.each(CASES)('renders the %s card shorter than the versioned one', async (_name, page) => {
    const [bare, versioned] = await Promise.all([pageSvg(page, brand, null), pageSvg(page, brand, '0.7.0')])
    expect(bare.length).toBeLessThan(versioned.length)
  })
})

describe('versionRows', () => {
  it.each([
    [null,    true,  0],
    [null,    false, 0],
    ['0.7.0', true,  2],
    ['0.7.0', false, 1]
  ])('returns the rows for version %s, divided %s, as %s nodes', (version, divided, count) => {
    expect(parts.versionRows(version, divided)).toHaveLength(count)
  })
})

describe('the data panel of a card with no rows', () => {
  const guide = CASES.find(([name]) => name === 'plain-guide')![1]

  it('draws no panel where no version fills it', async () => {
    const shell = vi.spyOn(parts, 'panelShell')
    await pageSvg(guide, brand, null)
    expect(shell).not.toHaveBeenCalled()
  })

  it('draws the panel around the version', async () => {
    const shell = vi.spyOn(parts, 'panelShell')
    await pageSvg(guide, brand, '0.7.0')
    expect(shell).toHaveBeenCalledOnce()
  })
})

describe('the data panel of a rule card with no version', () => {
  it('draws the panel around the family and pipeline rows', async () => {
    const rule  = CASES.find(([name]) => name === 'rule-cool')![1]
    const shell = vi.spyOn(parts, 'panelShell')
    await pageSvg(rule, brand, null)
    expect(shell).toHaveBeenCalledOnce()
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
