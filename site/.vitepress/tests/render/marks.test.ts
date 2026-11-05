import { formatHex } from 'culori'

import { SCHEME_SHADES, SCHEMES }               from '../../lib/shared/palette'
import { pointAt, renderTest, settle, styleOf } from './page'

const STACKED_ROW = '.glossary-folio-row:visible:has(.is-stacked):not([aria-current])'

describe('the body link', () => {
  renderTest('grows its nib stroke to 2.5em long and 4px deep under the pointer', async ({ visit }) => {
    const page = await visit('/guide/quick-start')
    const link = page.locator('.vp-doc a.body-link:not(:has(code))').first()
    await expect(styleOf(link, 'background-size')).resolves.toBe('100% 1px, 0px 4px')

    await pointAt(link)

    const stroke = 2.5 * Number.parseFloat(await styleOf(link, 'font-size'))
    await expect(styleOf(link, 'background-size')).resolves.toBe(`100% 1px, ${stroke}px 4px`)
  })
})

describe('the before-and-after toggle', () => {
  renderTest.for(SCHEMES)('sets the selected After pill in Celadon at weight 600 in the %s scheme', async (scheme, { visit }) => {
    const page = await visit('/rules/alignment/align-equals', { scheme })
    const card = page.locator('.fixture-card:has(button[data-side="after"])').first()
    const pill = card.locator('button[data-side="after"]')

    await card.locator('.fixture-card-summary').click()
    await settle(page)

    await expect(styleOf(pill, 'color').then(formatHex)).resolves.toBe(SCHEME_SHADES[scheme]['celadon-page-65'])
    await expect(styleOf(pill, 'font-weight')).resolves.toBe('600')
  })
})

describe('the callout mark', () => {
  renderTest.for([
    { behavior: 'sets the mark inline on a 640px page',         position: 'static',   width: 640 },
    { behavior: 'hangs the mark in the margin on a 641px page', position: 'absolute', width: 641 }
  ])('$behavior', async ({ position, width }, { visit }) => {
    const page = await visit('/integrations/editor', { width })

    await expect(styleOf(page.locator('.callout-mark').first(), 'position')).resolves.toBe(position)
  })
})

describe('the per-rule facet group', () => {
  renderTest('draws its line down the whole group once it opens', async ({ visit }) => {
    const page    = await visit('/reference/configuration')
    const section = page.locator('.per-rule-facets-section').first()
    await expect(styleOf(section, 'transform', '::before')).resolves.toBe('matrix(1, 0, 0, 0, 0, 0)')

    await section.locator('> .per-rule-facets-head').click()
    await settle(page)

    await expect(styleOf(section, 'transform', '::before')).resolves.toBe('matrix(1, 0, 0, 1, 0, 0)')
  })
})

describe('the pipeline spine', () => {
  renderTest('raises the tick under the pointer to the height of the spine', async ({ visit }) => {
    const page = await visit('/reference/pipeline-order')
    const tick = page.locator('.pipeline-order-spine li').nth(3)
    await expect(styleOf(tick, 'height', '::after')).resolves.toBe('4px')

    await pointAt(tick)

    await expect(styleOf(tick, 'height', '::after')).resolves.toBe(await styleOf(tick, 'height'))
  })
})

describe('the glossary deck', () => {
  renderTest('spreads a stacked tile to a 24px step while the pointer is on its row', async ({ visit }) => {
    const page = await visit('/reference/glossary')
    const row  = page.locator(STACKED_ROW).first()
    const tile = row.locator('.glossary-folio-row-tile').nth(1)
    await expect(styleOf(tile, 'right')).resolves.toBe('4px')

    await pointAt(row)

    await expect(styleOf(tile, 'right')).resolves.toBe('24px')
  })

  renderTest('holds the deck at its width while the tiles spread', async ({ visit }) => {
    const page = await visit('/reference/glossary')
    const row  = page.locator(STACKED_ROW).first()
    const deck = row.locator('.glossary-folio-row-deck')
    const rest = await styleOf(deck, 'width')

    await pointAt(row)

    await expect(styleOf(deck, 'width')).resolves.toBe(rest)
  })
})

describe('the narrow breakpoints', () => {
  renderTest.for([
    { narrow: 1, route: '/',                     selector: '.built-on-grid',                                variant: 'max-tiny',       wide: 3, width: 480 },
    { narrow: 1, route: '/',                     selector: '.landing-cta',                                  variant: 'max-card-stack', wide: 2, width: 720 },
    { narrow: 1, route: '/reference/exit-codes', selector: 'div:has(> nav[aria-label="Exit codes"])',       variant: 'max-tablet',     wide: 2, width: 760 },
    { narrow: 3, route: '/',                     selector: '.built-on-grid',                                variant: 'max-medium',     wide: 5, width: 820 },
    { narrow: 1, route: '/',                     selector: 'div:has(> section[aria-label="prose config"])', variant: 'max-desktop',    wide: 2, width: 960 }
  ])('$variant reaches a $width px page and stops one pixel past it', async ({ narrow, route, selector, wide, width }, { visit }) => {
    const tracksAt = async (at: number): Promise<number> => {
      const page = await visit(route, { width: at })
      return (await styleOf(page.locator(selector).first(), 'grid-template-columns')).split(' ').length
    }

    await expect(tracksAt(width)).resolves.toBe(narrow)
    await expect(tracksAt(width + 1)).resolves.toBe(wide)
  })
})
