import fs from 'node:fs'

import { defineLoader } from 'vitepress'

import * as walker  from '../fixtures/walker'
import { crateDir } from '../shared/paths'

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

const crate = crateDir(import.meta.url)

export default defineLoader({
  watch : walker.fixtureWatchGlobs(crate),
  load  : (): SandboxPool => {
    const cases: SandboxCase[] = []
    for (const { id, inputPath } of walker.walkFixtures(crate)) {
      const docs = walker.readFixtureDocs(inputPath)
      if (!docs?.sandbox) continue
      cases.push({ id, source: fs.readFileSync(inputPath, 'utf8'), title: docs.title ?? id })
    }
    if (cases.length === 0) throw new Error('sandbox pool is empty: no fixture carries sandbox = true')
    return { cases }
  }
})
