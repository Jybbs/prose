import axe                              from 'axe-core'
import { chromium }                     from 'playwright'
import type { Browser, BrowserContext } from 'playwright'

import { bundleContext, ORIGIN, PAGES } from './bundle'

interface Finding {
  id      : string
  targets : string[]
}

async function findings(context: BrowserContext, file: string): Promise<Finding[]> {
  const page = await context.newPage()
  try {
    await page.goto(`${ORIGIN}/${file}`, { waitUntil: 'networkidle' })
    await page.addScriptTag({ content: axe.source })
    return await page.evaluate(async () => {
      const { violations } = await (window as unknown as { axe: typeof axe }).axe.run()
      return violations.map(({ id, nodes }) => ({ id, targets: nodes.map(node => node.target.join(' ')) }))
    })
  }
  finally {
    await page.close()
  }
}

it('finds the built pages', () => {
  expect(PAGES, 'the sweep reads the bundle `mise run site:press` builds').toContain('reference/kit.html')
})

describe.each(['dark', 'light'] as const)('the built pages in the %s scheme', colorScheme => {
  let browser : Browser
  let context : BrowserContext

  beforeAll(async () => {
    browser = await chromium.launch()
    context = await bundleContext(browser, { colorScheme })
  })

  afterAll(() => browser.close())

  it.concurrent.each(PAGES)('%s has no axe violations', async file => {
    await expect(findings(context, file)).resolves.toStrictEqual([])
  })
})
