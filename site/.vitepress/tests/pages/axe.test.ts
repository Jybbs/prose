import axe                              from 'axe-core'
import { chromium }                     from 'playwright'
import type { Browser, BrowserContext } from 'playwright'

import { SCHEMES } from '../../lib/shared/palette'

import { bundleContext, PAGES } from './bundle'

interface Finding {
  id      : string
  targets : string[]
}

const FOCUSABLE       = 'a[href], button:not([disabled]), input, select, textarea, [tabindex]:not([tabindex="-1"])'
const FORCED_OVERFLOW = 'pre, code, table, td { letter-spacing: 0.5em; white-space: nowrap; }'
const NARROW_WIDTH    = 320

async function findings(context: BrowserContext, file: string): Promise<Finding[]> {
  const page = await context.newPage()
  try {
    await page.goto(`/${file}`, { waitUntil: 'networkidle' })
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

// Reads every rendered element a style lets scroll, whether or not its content
// overflows at this width, and names each with no tab stop of its own or inside it.
function unreachable(focusable: string): string[] {
  return [...document.body.querySelectorAll<HTMLElement>('*')]
    .filter(element => {
      const { overflowX, overflowY } = getComputedStyle(element)
      return [overflowX, overflowY].some(overflow => overflow === 'auto' || overflow === 'scroll')
        && element.getClientRects().length > 0
        && !element.matches(focusable)
        && element.querySelector(focusable) === null
    })
    .map(element => `${element.tagName.toLowerCase()}.${element.classList.value}`)
}

it('finds the built pages', () => {
  expect(PAGES, 'the sweep reads the bundle `mise run site:press` builds').toContain('reference/kit.html')
})

describe.each(SCHEMES)('the built pages in the %s scheme', colorScheme => {
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

describe('the scrolling regions at a forced overflow', () => {
  let browser : Browser
  let context : BrowserContext

  beforeAll(async () => {
    browser = await chromium.launch()
    context = await bundleContext(browser, { viewport: { height: 640, width: NARROW_WIDTH } })
  })

  afterAll(() => browser.close())

  it.concurrent.each(PAGES)('%s gives each a tab stop', async file => {
    const page = await context.newPage()
    try {
      await page.goto(`/${file}`, { waitUntil: 'networkidle' })
      await page.addStyleTag({ content: FORCED_OVERFLOW })
      await page.addScriptTag({ content: axe.source })
      await expect(page.evaluate(unreachable, FOCUSABLE)).resolves.toStrictEqual([])
      await expect(page.evaluate(async () => {
        const { violations } = await (window as unknown as { axe: typeof axe }).axe.run({ runOnly: ['scrollable-region-focusable'] })
        return violations.flatMap(({ nodes }) => nodes.map(node => node.target.join(' ')))
      })).resolves.toStrictEqual([])
    }
    finally {
      await page.close()
    }
  })
})
