import fs   from 'node:fs'
import path from 'node:path'

import config                 from '../../config'
import { REPO_URL }           from '../../lib/shared/constants'
import { pagesDir, repoRoot } from '../../lib/shared/paths'

const srcDir  = pagesDir(import.meta.url)
const pages   = fs.globSync('**/*.md', { cwd: srcDir }).sort()
const pattern = config.themeConfig?.editLink?.pattern

describe('the edit link', () => {
  it.each(pages)('maps %s to its file on main', page => {
    const file = path.relative(repoRoot(import.meta.url), path.join(srcDir, page))
    expect(String(pattern).replaceAll(':path', page)).toBe(`${REPO_URL}/edit/main/${file}`)
  })
})
