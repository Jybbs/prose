import fs   from 'node:fs'
import path from 'node:path'

import axe                              from 'axe-core'
import { chromium }                     from 'playwright'
import type { Browser, BrowserContext } from 'playwright'

interface Finding {
  id      : string
  targets : string[]
}

const DIST   = path.resolve(import.meta.dirname, '../../dist')
const ORIGIN = 'http://prose.test'
const PAGES  = fs.globSync('**/*.html', { cwd: DIST }).toSorted()

// Finds the built file a request reaches, under the clean URLs the site links.
function builtFile(pathname: string): string | undefined {
  const file = path.join(DIST, decodeURIComponent(pathname))
  return [file, `${file}.html`, path.join(file, 'index.html')]
    .find(candidate => fs.statSync(candidate, { throwIfNoEntry: false })?.isFile())
}

// Serves every request from the built bundle, which leaves no server to start
// and aborts any request to another origin.
async function bundleContext(browser: Browser, colorScheme: 'dark' | 'light'): Promise<BrowserContext> {
  const context = await browser.newContext({ colorScheme, reducedMotion: 'reduce' })
  await context.route('**/*', route => {
    const url = new URL(route.request().url())
    if (url.origin !== ORIGIN) return route.abort()
    const file = builtFile(url.pathname)
    return file ? route.fulfill({ path: file }) : route.fulfill({ status: 404 })
  })
  return context
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
  expect(PAGES).toContain('reference/kit.html')
})

describe.each(['dark', 'light'] as const)('the built pages in the %s scheme', colorScheme => {
  let browser : Browser
  let context : BrowserContext

  beforeAll(async () => {
    browser = await chromium.launch()
    context = await bundleContext(browser, colorScheme)
  })

  afterAll(() => browser.close())

  it.concurrent.each(PAGES)('%s has no axe violations', async file => {
    await expect(findings(context, file)).resolves.toStrictEqual([])
  })
})
