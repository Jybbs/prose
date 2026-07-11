import fs   from 'node:fs'
import path from 'node:path'

import { defineLoader } from 'vitepress'

import { fixturesDir } from '../shared/paths'

// A thematic case whose input shows field sorting, colon and equals column
// alignment, a docstring reshape, and blank-line normalization at once.
const SEED_CASE = 'thematic/dataclass_fields_and_docstring_reshape'

const seedInput = path.join(fixturesDir(import.meta.url), SEED_CASE, 'input.py')

interface SandboxSeed {
  source : string
}

declare const data: SandboxSeed
export { data }

export default defineLoader({
  watch : [seedInput],
  load  : (): SandboxSeed => ({ source: fs.readFileSync(seedInput, 'utf8') })
})
