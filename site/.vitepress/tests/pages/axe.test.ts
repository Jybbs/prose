import { chromium }                         from 'playwright'
import type { Browser, BrowserContext, Page } from 'playwright'

import { perScheme, SCHEMES } from '../../lib/shared/palette'
import type { Scheme }        from '../../lib/shared/palette'

import { audit }        from '../axe'
import type { Finding } from '../axe'

import { bundleContext, OPEN_CARD, open, PAGES, WIDE } from './bundle'

interface State {
  file  : string
  name  : string
  reach : (page: Page) => Promise<void>
  scope : string
}

const KICKER_BUTTON = 'button:has(> .kicker)'

// Opens the card the first element `selector` matches, through `act`.
const opensCard = (selector: string, act: 'click' | 'hover') => async (page: Page): Promise<void> => {
  await page.locator(selector).first()[act]()
  await page.locator(OPEN_CARD).waitFor()
}

// Each state a reader opens on a page, reached the way a reader reaches it, and
// the elements it opens, which the audit reads.
const STATES: readonly State[] = [
  {
    file  : 'rules/lint/step-narration.html',
    name  : 'a glossary card',
    reach : opensCard('.glossary-anchor[role="button"]', 'click'),
    scope : OPEN_CARD
  },
  {
    file  : 'index.html',
    name  : 'a hovered kicker link',
    reach : page => page.locator('a.kicker').hover(),
    scope : 'a.kicker'
  },
  {
    file  : 'sandbox/index.html',
    name  : 'a hovered kicker button',
    reach : page => page.locator(KICKER_BUTTON).hover(),
    scope : KICKER_BUTTON
  },
  {
    file  : 'rules/lint/step-narration.html',
    name  : 'a lint card',
    reach : opensCard('.lint-flag', 'click'),
    scope : OPEN_CARD
  },
  {
    file  : 'reference/pipeline-order.html',
    name  : 'a rule card',
    reach : opensCard('.pipeline-order-link[data-rule]', 'hover'),
    scope : OPEN_CARD
  },
  {
    file  : 'reference/cli.html',
    name  : 'a selected and a highlighted listbox row',
    reach : async page => {
      await page.locator('.run-summary-select-trigger').first().click()
      await page.locator('[role="option"][aria-selected="true"]:visible').waitFor()
      const row = page.locator('[role="option"]:not([aria-selected="true"]):visible').first()
      await row.hover()
      await row.and(page.locator('[data-highlighted]')).waitFor()
    },
    scope : '[role="listbox"]:visible'
  },
  {
    file  : 'integrations/editor.html',
    name  : 'a selected tab',
    reach : async page => {
      const tab = page.locator('[role="tab"]').nth(1)
      await tab.click()
      await tab.and(page.locator('[aria-selected="true"]')).waitFor()
    },
    scope : '[role="tablist"]'
  },
  {
    file  : 'index.html',
    name  : 'search results highlighting a match',
    reach : async page => {
      await page.locator('.VPNavBarSearchButton').click()
      await page.locator('.VPLocalSearchBox input').fill('align')
      await page.locator('.VPLocalSearchBox mark').first().waitFor()
    },
    scope : '.VPLocalSearchBox mark'
  }
]

let browser  : Browser
let contexts : Record<Scheme, Promise<BrowserContext>>

beforeAll(async () => {
  browser  = await chromium.launch()
  contexts = perScheme(colorScheme => bundleContext(browser, { colorScheme, viewport: WIDE }))
})

afterAll(() => browser.close())

const audits = new Map<string, Record<Scheme, Promise<Finding[]>>>()

// Loads the page once in each scheme, the first test to read it starting both
// loads, and holds what each load found for that scheme's test.
const audited = (file: string): Record<Scheme, Promise<Finding[]>> => audits.getOrInsertComputed(file, () =>
  perScheme(async scheme => open(await contexts[scheme], file, audit)))

it('finds the built pages', () => {
  expect(PAGES, 'the sweep reads the bundle `mise run site:press` builds').toContain('reference/kit.html')
})

describe.each(SCHEMES)('the built pages in the %s scheme', scheme => {
  it.concurrent.for(PAGES)('%s has no axe violations', async (file, { expect }) => {
    await expect(audited(file)[scheme]).resolves.toStrictEqual([])
  })
})

describe.each(SCHEMES)('the opened states in the %s scheme', scheme => {
  it.concurrent.for(STATES)('$name on $file has no axe violations', async (state, { expect }) => {
    await expect(open(await contexts[scheme], state.file, async page => {
      await state.reach(page)
      return audit(page, { scope: state.scope })
    })).resolves.toStrictEqual([])
  })
})
