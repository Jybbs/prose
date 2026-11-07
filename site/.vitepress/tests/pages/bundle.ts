import fs   from 'node:fs'
import path from 'node:path'

import type { Browser, BrowserContext, BrowserContextOptions, Page, ViewportSize } from 'playwright'
import { inject }                                                                  from 'vitest'

export const PAGES = fs.globSync('**/*.html', { cwd: path.resolve(import.meta.dirname, '../../dist') }).toSorted()

export const WIDE : ViewportSize = { height: 800, width: 1280 }

// Opens a context on the served bundle that aborts any request to another origin.
export async function bundleContext(browser: Browser, options: BrowserContextOptions): Promise<BrowserContext> {
  const baseURL = inject('siteUrl')
  const { origin } = new URL(baseURL)
  const context = await browser.newContext({ ...options, baseURL, reducedMotion: 'reduce' })
  await context.route('**/*', route =>
    new URL(route.request().url()).origin === origin ? route.continue() : route.abort())
  return context
}

// Opens the page in the context and returns what `measure` reads from it.
export async function open<T>(
  context : BrowserContext,
  file    : string,
  measure : (page: Page) => Promise<T>
): Promise<T> {
  const page = await context.newPage()
  try {
    await page.goto(`/${file}`, { waitUntil: 'networkidle' })
    return await measure(page)
  }
  finally {
    await page.close()
  }
}
