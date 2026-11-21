import { chromium }                         from 'playwright'
import type { Browser, BrowserContext, Page } from 'playwright'

import { seedUrl }            from '../../lib/sandbox/session'
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
const SANDBOX       = (await seedUrl('[rules.align-equals]\nmax-shfit = 4\n', 'x = 1\n'))!.slice(1)

// Sets `style` on a paragraph over a gradient, a ground `axe-core` leaves unresolved.
const onGradient = (style: string): string =>
  `<p style="background: linear-gradient(#fff, #fefefe); ${style}">Text</p>`

const FADED = `<div style="opacity: .25">${onGradient('color: #000')}</div>`
// Texts the pixel fallback reads, each on a ground `axe-core` leaves unresolved,
// the two grays holding 4.44:1 and 4.54:1 on white.
const FALLBACK = [
  { fails: true,  html: onGradient('color: #777'),                         text: 'gray under the floor' },
  { fails: false, html: onGradient('color: #767676'),                      text: 'gray at the floor' },
  { fails: true,  html: FADED,                                             text: 'text at 25% opacity' },
  { fails: true,  html: onGradient('-webkit-text-fill-color: #ddd'),       text: 'a light text fill' },
  { fails: true,  html: '<p style="color: #eee">·</p>',                    text: 'a light glyph' },
  { fails: false, html: '<p aria-hidden="true" style="color: #eee">·</p>', text: 'a hidden glyph' },
  {
    fails : true,
    html  : '<p aria-hidden="true" style="background: linear-gradient(#fff, #fefefe); color: #eee">Text</p>',
    text  : 'a hidden word over a gradient'
  },
  {
    fails : false,
    html  : '<p aria-hidden="true"><span style="color: #eee">·</span></p>',
    text  : 'a glyph inside hidden text'
  },
  {
    fails : true,
    html  : `<div style="opacity: .9">${onGradient('color: rgb(0 0 0 / .5)')}</div>`,
    text  : 'half-alpha text under a light fade'
  },
  {
    fails : true,
    html  : '<div style="opacity: .8; background: linear-gradient(#646464, #656565)">'
          + '<p style="color: #000">Text</p></div>',
    text  : 'black on a faded gray ground'
  },
  {
    fails : false,
    html  : '<div style="background: #000">'
          + '<p style="opacity: .5; background: linear-gradient(#fff, #fefefe); color: #000">Text</p></div>',
    text  : 'black on a faded white ground over black'
  }
]

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
    file  : SANDBOX,
    name  : 'a flag card',
    reach : opensCard('.sandbox-toml-display .lint-flag', 'hover'),
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

describe('the pixel fallback', () => {
  it.concurrent.for(FALLBACK)('reads $text as failing: $fails', async ({ fails, html }, { expect }) => {
    const page = await (await contexts.light).newPage()
    try {
      await page.setContent(`<main>${html}</main>`)
      expect((await audit(page, { runOnly: ['color-contrast'] })).length > 0).toBe(fails)
    }
    finally {
      await page.close()
    }
  })
})
