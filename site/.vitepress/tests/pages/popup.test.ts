import { chromium }                           from 'playwright'
import type { Browser, BrowserContext, Page } from 'playwright'

import { bundleContext, ORIGIN } from './bundle'

const OPEN_CARD = '[role="dialog"]:visible'

describe('the cards under touch emulation at a phone width', () => {
  let browser : Browser
  let context : BrowserContext
  let page    : Page

  beforeAll(async () => {
    browser = await chromium.launch()
    context = await bundleContext(browser, { hasTouch: true, isMobile: true, viewport: { height: 800, width: 390 } })
  })

  beforeEach(async () => {
    page = await context.newPage()
    await page.goto(`${ORIGIN}/rules/lint/step-narration.html`, { waitUntil: 'networkidle' })
  })

  afterEach(() => page.close())

  afterAll(() => browser.close())

  it('opens a glossary card from a tap and moves focus to its link', async () => {
    await page.locator('.glossary-anchor[role="button"]').first().tap()
    await page.locator(OPEN_CARD).waitFor()
    await page.waitForFunction(() => document.activeElement?.textContent.includes('Read more'))
  })

  it('opens a lint card from a tap and moves focus into it', async () => {
    await page.locator('.lint-flag').first().tap()
    await page.locator(OPEN_CARD).waitFor()
    await page.waitForFunction(() => document.activeElement?.getAttribute('role') === 'dialog')
  })

  it('closes a lint card from a tap outside it', async () => {
    await page.locator('.lint-flag').first().tap()
    await page.locator(OPEN_CARD).waitFor()
    await page.locator('h1').tap()
    await page.locator(OPEN_CARD).waitFor({ state: 'hidden' })
  })
})
