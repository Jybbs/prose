import { defineLoader } from 'vitepress'

import { readFixtureToggle } from '../fixtures/toggle'
import * as walker           from '../fixtures/walker'
import { crateDir }          from '../shared/paths'

interface RuleExample {
  case  : string
  title : string
}

export interface RuleFixtureSet {
  canonical : string
  examples  : readonly RuleExample[]
}

type RuleFixturesData = Record<string, RuleFixtureSet>

const crate = crateDir(import.meta.url)

const sortKey = (title: string): string => title.replace(/^`+/, '')

declare const data: RuleFixturesData
export { data }

export default defineLoader({
  watch: walker.fixtureWatchGlobs(crate),
  async load(): Promise<RuleFixturesData> {
    type Pending        = { canonical: string | null, examples: PendingExample[] }
    type PendingExample = RuleExample & { inputPath: string }

    const byRule: Record<string, Pending> = {}
    for (const { rule, caseName, inputPath } of walker.walkFixtures(crate)) {
      const docs = walker.readFixtureDocs(inputPath)
      if (docs === undefined) continue
      const set   = (byRule[rule] ??= { canonical: null, examples: [] })
      const title = walker.fixtureTitle(docs)
      if (docs.canonical === true) {
        set.canonical = caseName
      } else if (docs.previewable === true && title) {
        set.examples.push({ case: caseName, inputPath, title })
      }
    }

    const out: RuleFixturesData = {}
    for (const [rule, { canonical, examples }] of Object.entries(byRule)) {
      if (canonical === null) continue
      const ranked = await Promise.all(examples.map(async ex => ({
        ex,
        hasToggle: (await readFixtureToggle(ex.inputPath)).hasToggle
      })))
      ranked.sort((a, b) =>
        Number(b.hasToggle) - Number(a.hasToggle) ||
        sortKey(a.ex.title).localeCompare(sortKey(b.ex.title)))
      out[rule] = {
        canonical,
        examples: ranked.map(({ ex }) => ({ case: ex.case, title: ex.title }))
      }
    }
    return out
  }
})
