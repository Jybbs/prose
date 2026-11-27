import { chromium }                           from 'playwright'
import type { Browser, BrowserContext, Page } from 'playwright'

import { seedUrl } from '../../../site/.vitepress/lib/sandbox/session'

import { bundleContext, OPEN_CARD, open, WIDE } from './bundle'

const TOML    = 'code-line-length = 88\n\n[rules.align-equals]\nmax-shfit = 4\n\n[rules.align-colons]\nmax-shfit = 4\n'
const SANDBOX = (await seedUrl(TOML, 'x = 1\n'))!.slice(1)
const EDITOR  = '.sandbox-toml textarea'

// A point on the `at`th misspelled key, relative to the text area.
const keySpot = (page: Page, at: number) => page.locator(EDITOR).evaluate((area, nth) => {
  const text = (area as HTMLTextAreaElement).value
  let index = -1
  for (let seen = 0; seen <= nth; seen++) index = text.indexOf('max-shfit', index + 1)
  const row   = text.slice(0, index).split('\n').length - 1
  const style = getComputedStyle(area)
  const line  = parseFloat(style.lineHeight)
  return { x: parseFloat(style.paddingLeft) + 20, y: parseFloat(style.paddingTop) + row * line + line / 2 }
}, at)

// How far the open card's nearer edge sits from the first key's row.
const cardGap = async (page: Page) => {
  const card = (await page.locator(OPEN_CARD).boundingBox())!
  const area = (await page.locator(EDITOR).boundingBox())!
  const row  = area.y + (await keySpot(page, 0)).y
  return Math.min(Math.abs(card.y - row), Math.abs(card.y + card.height - row))
}

let browser : Browser
beforeAll(async () => { browser = await chromium.launch({ args: ['--mute-audio'] }) })
afterAll(() => browser.close())

describe('flag cards under touch', () => {
  let context : BrowserContext
  let page    : Page
  const label  = () => page.evaluate(() =>
    document.querySelector('[role="dialog"]')?.getAttribute('aria-label') ?? null)
  const active = () => page.evaluate(() => document.activeElement?.tagName)
  const spot   = (key: number) => keySpot(page, key)
  const tapKey = async (key: number) => {
    await page.locator(EDITOR).tap({ position: await spot(key) })
    await page.waitForTimeout(500)
  }
  const clear = async () => {
    await page.locator(EDITOR).tap({ position: { x: 250, y: 24 } })
    await page.waitForTimeout(500)
    await expect(label()).resolves.toBe(null)
  }
  const first  = 'rules.align-equals.max-shfit'
  const second = 'rules.align-colons.max-shfit'

  beforeAll(async () => {
    const viewport = { height: 800, width: 390 }
    context = await bundleContext(browser, { hasTouch: true, isMobile: true, viewport })
  })
  beforeEach(async () => {
    page = await context.newPage()
    await page.goto(`/${SANDBOX}`, { waitUntil: 'networkidle' })
    await page.locator('.sandbox-toml-display .lint-flag').nth(1).waitFor()
  })
  afterEach(() => page.close())

  const focusEditor = async () => {
    await page.locator('.sandbox-toml-display').tap({ position: { x: 250, y: 24 } })
    await expect.poll(active).toBe('TEXTAREA')
    await page.waitForTimeout(400)
    await expect(label()).resolves.toBe(null)
  }

  it('opens each flag card after the editor holds focus', async () => {
    await focusEditor()
    await tapKey(0)
    await expect(label()).resolves.toBe(first)
    await tapKey(0)
    await expect(label()).resolves.toBe(first)
    await clear()
    await tapKey(1)
    await expect(label()).resolves.toBe(second)
    await tapKey(1)
    await expect(label()).resolves.toBe(second)
    await tapKey(0)
    await expect(label()).resolves.toBe(first)
    await expect(active()).resolves.toBe('TEXTAREA')
    await expect(cardGap(page)).resolves.toBeLessThan(40)
    await expect(page.locator('.sandbox-toml .sandbox-code-editor .lint-flag').count()).resolves.toBe(0)
  })

  it('opens the card from the tap that focuses the editor', async () => {
    await page.locator('.sandbox-toml-display .lint-flag').first().tap()
    await expect.poll(active).toBe('TEXTAREA')
    await page.waitForTimeout(700)
    await expect(label()).resolves.toBe(first)
    await expect(cardGap(page)).resolves.toBeLessThan(40)
    await clear()
    await tapKey(1)
    await expect(label()).resolves.toBe(second)
  })

  it('opens each flag card where the engine reports a tap\'s click as a mouse\'s', async () => {
    await page.close()
    page = await context.newPage()
    await page.addInitScript(() => {
      const read = Object.getOwnPropertyDescriptor(PointerEvent.prototype, 'pointerType')!.get!
      Object.defineProperty(PointerEvent.prototype, 'pointerType', {
        get() { return this.type === 'click' ? 'mouse' : read.call(this) }
      })
    })
    await page.goto(`/${SANDBOX}`, { waitUntil: 'networkidle' })
    await page.locator('.sandbox-toml-display .lint-flag').nth(1).waitFor()
    await focusEditor()
    await tapKey(0)
    await expect(label()).resolves.toBe(first)
    await clear()
    await tapKey(1)
    await expect(label()).resolves.toBe(second)
  })

  it('closes on a tap off the key, and typing closes', async () => {
    await focusEditor()
    await tapKey(0)
    await expect(label()).resolves.toBe(first)
    await page.locator(EDITOR).tap({ position: { x: 250, y: 24 } })
    await page.waitForTimeout(500)
    await expect(label()).resolves.toBe(null)
    await tapKey(0)
    await expect(label()).resolves.toBe(first)
    await page.keyboard.type('x')
    await page.keyboard.press('Backspace')
    await page.waitForTimeout(600)
    await expect(label()).resolves.toBe(null)
  })

  it('closes the card as the notices under it change', async () => {
    await focusEditor()
    // The page clock holds the 250 ms format debounce, so the tap lands before
    // the notices change, and a second past it lets them land.
    await page.clock.install()
    await page.keyboard.type('x')
    await page.locator(EDITOR).tap({ position: await spot(0) })
    await expect.poll(label).toBe(first)
    await page.clock.runFor(1000)
    await expect.poll(label).toBe(null)
  })

  it('applies a replacement from the card', async () => {
    await focusEditor()
    await tapKey(0)
    await page.locator(`${OPEN_CARD} button`).first().tap()
    await page.waitForTimeout(500)
    await expect(page.locator(EDITOR).inputValue()).resolves.toContain('[rules.align-equals]\nmax-shift = 4')
    await expect(label()).resolves.toBe(null)
    await tapKey(0)
    await expect(label()).resolves.toBe(second)
  })

  it('keeps the card open through a tap on its own text', async () => {
    await focusEditor()
    await tapKey(0)
    await page.locator(`${OPEN_CARD} p`).first().tap()
    await page.waitForTimeout(500)
    await expect(label()).resolves.toBe(first)
    await expect(active()).resolves.toBe('TEXTAREA')
  })

  it('closes with the editor, and a hover after it closes on its leave', async () => {
    await focusEditor()
    await tapKey(0)
    await expect(label()).resolves.toBe(first)
    await page.locator(EDITOR).evaluate(area => { (area as HTMLElement).blur() })
    await expect.poll(label).toBe(null)
    await page.locator('.sandbox-toml-display .lint-flag').nth(1).waitFor()
    const flag = (await page.locator('.sandbox-toml-display .lint-flag').nth(1).boundingBox())!
    await page.mouse.move(flag.x + flag.width / 2, flag.y + flag.height / 2)
    await expect.poll(label).toBe(second)
    await page.mouse.move(2, 2)
    await expect.poll(label).toBe(null)
  })

  it('closes on Escape and opens on the next tap', async () => {
    await focusEditor()
    await tapKey(0)
    await expect(label()).resolves.toBe(first)
    await page.keyboard.press('Escape')
    await expect.poll(label).toBe(null)
    await tapKey(0)
    await expect(label()).resolves.toBe(first)
  })
})

describe('a pointer', () => {
  it('opens nothing from a click on a key in the editor', async () => {
    const context = await bundleContext(browser, { viewport: WIDE })
    await open(context, SANDBOX, async page => {
      await page.locator('.sandbox-toml-display .lint-flag').first().click()
      await page.waitForTimeout(500)
      await expect(page.locator(OPEN_CARD).count()).resolves.toBe(0)
      await page.locator(EDITOR).click({ position: await keySpot(page, 0) })
      await page.waitForTimeout(300)
      await expect(page.locator(OPEN_CARD).count()).resolves.toBe(0)
    })
  })
})
