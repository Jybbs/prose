import fs   from 'node:fs'
import path from 'node:path'

import config                 from '../../config'
import { REPO_URL }           from '../../lib/shared/constants'
import { pagesDir, repoRoot } from '../../lib/shared/paths'

const EDIT    = `${REPO_URL}/edit/main/`
const PAGES   = fs.globSync('**/*.md', { cwd: pagesDir(import.meta.url) }).sort()
const PATTERN = config.themeConfig?.editLink?.pattern

describe('the edit link', () => {
  it('opens on the repository edit URL for main', () => {
    expect(String(PATTERN).startsWith(EDIT)).toBe(true)
  })

  it.each(PAGES)('maps %s to a tracked file', page => {
    const file = String(PATTERN).slice(EDIT.length).replace(':path', page)
    expect(fs.existsSync(path.join(repoRoot(import.meta.url), file))).toBe(true)
  })
})
