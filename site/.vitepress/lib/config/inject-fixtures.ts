import fs   from 'node:fs'
import path from 'node:path'

import type { PageData } from 'vitepress'

import { pageCaseIds, type PageFixtureSets } from '../fixtures/page-cases'
import { fixtureEntries }                    from '../fixtures/render'

// Injects the cases a page renders into that page's frontmatter at build
// time, leaving the fixture corpus out of every page's download.
export async function injectFixtures(
  pageData : PageData,
  crate    : string,
  sets     : PageFixtureSets,
  srcDir   : string
): Promise<void> {
  if (!pageData.filePath) return
  const source = fs.readFileSync(path.join(srcDir, pageData.filePath), 'utf8')
  const ids    = pageCaseIds(source, sets)
  if (ids.length > 0) pageData.frontmatter.fixtures = await fixtureEntries(crate, ids)
}
