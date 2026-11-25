import fs   from 'node:fs'
import path from 'node:path'

import config                 from '../../../site/.vitepress/config'
import { REPO_URL }           from '../../../site/.vitepress/lib/shared/constants'
import { pagesDir, repoRoot } from '../../../site/.vitepress/lib/shared/paths'

import type { readPublishedVersion } from '../../../site/.vitepress/lib/shared/published-version'

vi.mock('../../../site/.vitepress/lib/shared/published-version', () => ({
  readPublishedVersion: vi.fn<typeof readPublishedVersion>().mockResolvedValue('9.8.7')
}))

const srcDir  = pagesDir(import.meta.url)
const pages   = fs.globSync('**/*.md', { cwd: srcDir }).sort()
const pattern = config.themeConfig?.editLink?.pattern

describe('the edit link', () => {
  it.each(pages)('maps %s to its file on main', page => {
    const file = path.relative(repoRoot(import.meta.url), path.join(srcDir, page))
    expect(String(pattern).replaceAll(':path', page)).toBe(`${REPO_URL}/edit/main/${file}`)
  })
})
