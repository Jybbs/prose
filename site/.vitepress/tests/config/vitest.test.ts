import fs   from 'node:fs'
import path from 'node:path'

import config       from '../../../vitest.config'
import { repoRoot } from '../../lib/shared/paths'

const root     = repoRoot(import.meta.url)
const triggers = (config.test?.forceRerunTriggers ?? [])
  .filter(glob => glob.startsWith(root))
  .map(glob => path.relative(root, glob))

describe('the force-rerun triggers', () => {
  it('declare the inputs the docs project reads from disk', () => {
    expect(triggers.length).toBeGreaterThan(0)
  })

  it.each(triggers)('%s matches a file in the repository', glob => {
    expect(fs.globSync(glob, { cwd: root, exclude: ['**/node_modules/**'] }).length).toBeGreaterThan(0)
  })
})
