import { chromium }                           from 'playwright'
import type { Browser, BrowserContext, Page } from 'playwright'

import { bundleContext } from './bundle'

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
    await page.goto('/rules/lint/step-narration.html', { waitUntil: 'networkidle' })
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

describe('the lint cards from a keyboard at a desktop width', () => {
  let browser : Browser
  let context : BrowserContext
  let page    : Page

  const active = () => page.evaluate(() => (document.activeElement as HTMLElement | null)?.dataset.probe)

  beforeAll(async () => {
    browser = await chromium.launch()
    context = await bundleContext(browser, { viewport: { height: 900, width: 1280 } })
  })

  beforeEach(async () => {
    page = await context.newPage()
    await page.goto('/rules/lint/step-narration.html', { waitUntil: 'networkidle' })
    await page.locator('.lint-flag').first().focus()
    await page.evaluate(() => {
      const tabbable = [...document.querySelectorAll<HTMLElement>('a[href], button, [tabindex="0"]')]
      const at       = tabbable.indexOf(document.activeElement as HTMLElement)
      tabbable[at].dataset.probe     = 'flag'
      tabbable[at + 1].dataset.probe = 'after'
    })
  })

  afterEach(() => page.close())

  afterAll(() => browser.close())

  it.each(['Enter', 'Space'])('opens from %s, moves focus inside, and Escape returns focus to the flag', async key => {
    await page.keyboard.press(key)
    await page.locator(OPEN_CARD).waitFor()
    await page.waitForFunction(() => document.activeElement?.getAttribute('role') === 'dialog')
    await page.keyboard.press('Escape')
    await page.locator(OPEN_CARD).waitFor({ state: 'hidden' })
    await expect.poll(active).toBe('flag')
  })

  it('closes on a second click of the flag it opened from', async () => {
    await page.locator('[data-probe="flag"]').click()
    await page.locator(OPEN_CARD).waitFor()
    await page.locator('[data-probe="flag"]').click()
    await page.locator(OPEN_CARD).waitFor({ state: 'hidden' })
  })

  it('moves Tab out of a card to the tabbable after its flag', async () => {
    await page.keyboard.press('Enter')
    await page.waitForFunction(() => document.activeElement?.getAttribute('role') === 'dialog')
    await page.keyboard.press('Tab')
    await expect.poll(active).toBe('after')
  })

  it('moves Shift+Tab out of a card to its flag', async () => {
    await page.keyboard.press('Enter')
    await page.waitForFunction(() => document.activeElement?.getAttribute('role') === 'dialog')
    await page.keyboard.press('Shift+Tab')
    await expect.poll(active).toBe('flag')
  })
})

describe('the rule cards from a pointer at a desktop width', () => {
  let browser : Browser
  let context : BrowserContext
  let page    : Page

  const LINK = '.pipeline-order-link[data-rule]'

  beforeAll(async () => {
    browser = await chromium.launch()
    context = await bundleContext(browser, { viewport: { height: 900, width: 1280 } })
  })

  beforeEach(async () => {
    page = await context.newPage()
    await page.goto('/reference/pipeline-order.html', { waitUntil: 'networkidle' })
    await page.locator(LINK).first().hover()
    await page.locator(OPEN_CARD).waitFor()
  })

  afterEach(() => page.close())

  afterAll(() => browser.close())

  it('opens on a hover and leaves focus on the page', async () => {
    await page.waitForTimeout(200)
    await expect(page.evaluate(() => document.activeElement?.getAttribute('role'))).resolves.not.toBe('dialog')
  })

  it('places the card against the hovered link', async () => {
    const link = (await page.locator(LINK).first().boundingBox())!
    await expect.poll(async () => {
      const card = (await page.locator(OPEN_CARD).boundingBox())!
      return Math.min(Math.abs(card.y - link.y - link.height), Math.abs(link.y - card.y - card.height))
    }).toBeLessThan(40)
  })
})

describe('the tab orientation against the stacked layout', () => {
  let browser : Browser

  beforeAll(async () => { browser = await chromium.launch() })

  afterAll(() => browser.close())

  it.each(['editor', 'shell-completions'].flatMap(name => [
    { name, orientation: 'horizontal', width: 420 },
    { name, orientation: 'horizontal', width: 720 },
    { name, orientation: 'vertical',   width: 721 }
  ]))('reads $orientation on the $name page loaded at $width px', async ({ name, orientation, width }) => {
    const context = await bundleContext(browser, { viewport: { height: 900, width } })
    const page    = await context.newPage()
    await page.goto(`/integrations/${name}.html`, { waitUntil: 'networkidle' })
    // Waits for the client to mount, since the server renders the wide layout's
    // orientation and each read waits on the main thread while it hydrates.
    await page.waitForFunction(() => '__vue_app__' in (document.querySelector('#app') ?? {}))
    await expect.poll(() => page.locator('[role="tablist"]').first().getAttribute('aria-orientation')).toBe(orientation)
    await context.close()
  })
})

describe('the composition decks on a rule page', () => {
  let browser : Browser

  beforeAll(async () => { browser = await chromium.launch() })

  afterAll(() => browser.close())

  it('measures the deck of every card', async () => {
    const context = await bundleContext(browser, { viewport: { height: 900, width: 1280 } })
    const page    = await context.newPage()
    await page.goto('/rules/ordering/alphabetize-siblings.html', { waitUntil: 'networkidle' })
    // Waits for the client to mount, since the server renders every deck and only
    // the mounted cards measure theirs, while each read waits on the main thread
    // through hydration.
    await page.waitForFunction(() => '__vue_app__' in (document.querySelector('#app') ?? {}))
    const unmeasured = page.locator('li.fixture-card:has(.composition-cards-bar):not([style*="--deck-bar-h"])')
    expect(await page.locator('.composition-cards-bar').count()).toBeGreaterThan(1)
    await expect.poll(() => unmeasured.count()).toBe(0)
    await context.close()
  })
})
