import fs   from 'node:fs'
import path from 'node:path'

import type { Browser, BrowserContext, BrowserContextOptions } from 'playwright'

const DIST          = path.resolve(import.meta.dirname, '../../dist')
export const ORIGIN = 'http://prose.test'
export const PAGES  = fs.globSync('**/*.html', { cwd: DIST }).toSorted()

// Finds the built file a request reaches, under the clean URLs the site links.
function builtFile(pathname: string): string | undefined {
  const file = path.join(DIST, decodeURIComponent(pathname))
  return [file, `${file}.html`, path.join(file, 'index.html')]
    .find(candidate => fs.statSync(candidate, { throwIfNoEntry: false })?.isFile())
}

// Serves every request from the built bundle, which leaves no server to start
// and aborts any request to another origin.
export async function bundleContext(browser: Browser, options: BrowserContextOptions): Promise<BrowserContext> {
  const context = await browser.newContext({ ...options, reducedMotion: 'reduce' })
  await context.route('**/*', route => {
    const url = new URL(route.request().url())
    if (url.origin !== ORIGIN) return route.abort()
    const file = builtFile(url.pathname)
    return file ? route.fulfill({ path: file }) : route.fulfill({ status: 404 })
  })
  return context
}
