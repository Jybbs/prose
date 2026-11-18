import fs   from 'node:fs'
import path from 'node:path'

import * as paths         from '../../lib/shared/paths.ts'
import { supportTest }    from '../support.ts'
import { CRATE, READERS } from '../../vitest.config.ts'

const ROOT    = paths.repoRoot(import.meta.url)
const RUNNER  = /\b(runProse|readProseUnder)\(/
const RUNNERS = fs.globSync('site/.vitepress/lib/**/*.ts', { cwd: ROOT })
  .filter(file => !file.endsWith('/paths.ts') && RUNNER.test(fs.readFileSync(path.join(ROOT, file), 'utf8')))
  .toSorted()

describe('READERS', () => {
  supportTest.for(RUNNERS)('reads the crate sources for %s, which runs the binary', file => {
    expect(READERS).toHaveProperty([file], expect.arrayContaining([CRATE]))
  })
})
