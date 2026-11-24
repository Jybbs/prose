import fs   from 'node:fs'
import path from 'node:path'

import * as walker from '../../site/.vitepress/lib/fixtures/walker'
import * as paths  from '../../site/.vitepress/lib/shared/paths'

export const CRATE    = paths.crateDir(import.meta.url)
export const FIXTURES = paths.fixturesDir(import.meta.url)
export const CASES    = [...walker.walkFixtures(FIXTURES)]

// Reports whether a case carries the named sidecar beside its input.
export function hasSidecar(inputPath: string, file: string): boolean {
  return fs.existsSync(path.join(path.dirname(inputPath), file))
}
