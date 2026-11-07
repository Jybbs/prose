import { chromium }                     from 'playwright'
import type { Browser, BrowserContext } from 'playwright'

import { bundleContext, PAGES } from './bundle'

// Lists each inline code chip whose box splits across lines or carries the page
// past the viewport's width.
async function splitChips(context: BrowserContext, file: string): Promise<string[]> {
  const page = await context.newPage()
  try {
    await page.goto(`/${file}`, { waitUntil: 'networkidle' })
    return await page.evaluate(() => [...document.querySelectorAll<HTMLElement>('.vp-doc :not(pre) > code')]
      .filter(chip => chip.getClientRects().length > 1
        || (document.documentElement.scrollWidth > innerWidth && chip.getBoundingClientRect().right > innerWidth))
      .map(chip => chip.textContent))
  }
  finally {
    await page.close()
  }
}

// Lists each inline code chip whose following punctuation opens the next line.
async function strandedPunctuation(context: BrowserContext, file: string): Promise<string[]> {
  const page = await context.newPage()
  try {
    await page.goto(`/${file}`, { waitUntil: 'networkidle' })
    return await page.evaluate(() => [...document.querySelectorAll<HTMLElement>('.vp-doc :not(pre) > code')]
      .filter(chip => {
        let at : Node = chip
        while (!at.nextSibling && at.parentElement?.matches('a, em, strong')) at = at.parentElement
        const text = at.nextSibling
        if (chip.getClientRects().length === 0 || !(text instanceof Text) || !/^[,.;:!?)\]}'"’”…]/.test(text.data)) return false
        const range = document.createRange()
        range.setStart(text, 0)
        range.setEnd(text, 1)
        return range.getBoundingClientRect().top >= chip.getBoundingClientRect().bottom - 1
      })
      .map(chip => chip.textContent))
  }
  finally {
    await page.close()
  }
}

describe.each([1280, 360])('the built pages at %d px', width => {
  let browser : Browser
  let context : BrowserContext

  beforeAll(async () => {
    browser = await chromium.launch()
    context = await bundleContext(browser, { viewport: { height: 800, width } })
  })

  afterAll(() => browser.close())

  it.concurrent.each(PAGES)('%s holds each inline code chip in one box inside the page', async file => {
    await expect(splitChips(context, file)).resolves.toStrictEqual([])
  })

  it.concurrent.each(PAGES)('%s fits the viewport\'s width', async file => {
    const page = await context.newPage()
    try {
      await page.goto(`/${file}`, { waitUntil: 'networkidle' })
      await expect(page.evaluate(() => document.documentElement.scrollWidth - innerWidth)).resolves.toBeLessThanOrEqual(0)
    }
    finally {
      await page.close()
    }
  })

  it.concurrent.each(PAGES)('%s holds the punctuation after a chip on its line', async file => {
    await expect(strandedPunctuation(context, file)).resolves.toStrictEqual([])
  })
})
