import { chromium }                     from 'playwright'
import type { Browser, BrowserContext } from 'playwright'

import { bundleContext, PAGES } from './bundle'

// Lists each inline code chip whose box splits across lines.
async function splitChips(context: BrowserContext, file: string): Promise<string[]> {
  const page = await context.newPage()
  try {
    await page.goto(`/${file}`, { waitUntil: 'networkidle' })
    return await page.evaluate(() => [...document.querySelectorAll<HTMLElement>('.vp-doc :not(pre) > code')]
      .filter(chip => chip.getClientRects().length > 1)
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

  it.concurrent.each(PAGES)('%s holds each inline code chip in one box', async file => {
    await expect(splitChips(context, file)).resolves.toStrictEqual([])
  })
})
