import fs   from 'node:fs'
import path from 'node:path'

import config                 from '../../config'
import { REPO_URL }           from '../../lib/shared/constants'
import { pagesDir, repoRoot } from '../../lib/shared/paths'

const PAGES   = pagesDir(import.meta.url)
const PATTERN = config.themeConfig?.editLink?.pattern

describe('the edit link', () => {
  it.each(fs.globSync('**/*.md', { cwd: PAGES }).sort())('maps %s to its file on main', page => {
    const file = path.relative(repoRoot(import.meta.url), path.join(PAGES, page))
    expect(String(PATTERN).replaceAll(':path', page)).toBe(`${REPO_URL}/edit/main/${file}`)
  })
})
