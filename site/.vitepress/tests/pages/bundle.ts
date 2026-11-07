import fs   from 'node:fs'
import path from 'node:path'

import type { Browser, BrowserContext, BrowserContextOptions } from 'playwright'
import { inject }                                              from 'vitest'

export const PAGES = fs.globSync('**/*.html', { cwd: path.resolve(import.meta.dirname, '../../dist') }).toSorted()

// Opens a context on the served bundle that aborts any request to another origin.
export async function bundleContext(browser: Browser, options: BrowserContextOptions): Promise<BrowserContext> {
  const baseURL = inject('siteUrl')
  const { origin } = new URL(baseURL)
  const context = await browser.newContext({ ...options, baseURL, reducedMotion: 'reduce' })
  await context.route('**/*', route =>
    new URL(route.request().url()).origin === origin ? route.continue() : route.abort())
  return context
}
