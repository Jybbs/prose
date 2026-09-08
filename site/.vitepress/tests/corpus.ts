import fs   from 'node:fs'
import path from 'node:path'

import * as walker  from '../lib/fixtures/walker'
import { crateDir } from '../lib/shared/paths'

export const CRATE = crateDir(import.meta.url)
export const CASES = [...walker.walkFixtures(CRATE)]

// Reports whether a case carries the named sidecar beside its input.
export function hasSidecar(inputPath: string, file: string): boolean {
  return fs.existsSync(path.join(path.dirname(inputPath), file))
}
