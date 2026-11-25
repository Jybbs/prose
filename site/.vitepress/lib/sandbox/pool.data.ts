import fs from 'node:fs'

import { defineLoader } from 'vitepress'

import * as walker     from '../fixtures/walker.ts'
import { fixturesDir } from '../shared/paths.ts'

export interface SandboxCase {
  id     : string
  source : string
  title  : string
}

interface SandboxPool {
  cases : readonly SandboxCase[]
}

declare const data: SandboxPool
export { data }

const fixturesRoot = fixturesDir(import.meta.url)

export default defineLoader({
  watch : walker.fixtureWatchGlobs(fixturesRoot),
  load  : (): SandboxPool => {
    const cases: SandboxCase[] = []
    for (const { id, inputPath } of walker.walkFixtures(fixturesRoot)) {
      const docs = walker.readFixtureDocs(inputPath)
      if (!docs?.sandbox) continue
      const title = walker.fixtureTitle(docs) ?? id
      cases.push({ id, source: fs.readFileSync(inputPath, 'utf8'), title })
    }
    if (cases.length === 0) throw new Error('sandbox pool is empty: no fixture carries sandbox = true')
    return { cases }
  }
})
