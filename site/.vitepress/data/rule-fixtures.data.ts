import path from 'node:path'

import { defineLoader } from 'vitepress'

import { FIXTURES_DIR, META_FILE, readFixtureDocs, walkFixtures } from '../lib/fixtures/walker'
import { repoRoot } from '../lib/shared/paths'

interface RuleExample {
  case  : string
  title : string
}

interface RuleFixtureSet {
  canonical : string
  examples  : readonly RuleExample[]
}

type RuleFixturesData = Record<string, RuleFixtureSet>

const root        = repoRoot(import.meta.url)
const fixturesDir = path.join(root, FIXTURES_DIR)

const sortKey = (title: string): string => title.replace(/^`+/, '')

declare const data: RuleFixturesData
export { data }

export default defineLoader({
  watch: [`${fixturesDir}/*/*/${META_FILE}`],
  async load(): Promise<RuleFixturesData> {
    type Pending = { canonical: string | null, examples: RuleExample[] }

    const byRule: Record<string, Pending> = {}
    for (const { rule, caseName, inputPath } of walkFixtures(root)) {
      const docs = readFixtureDocs(inputPath)
      if (docs === undefined) continue
      const set   = (byRule[rule] ??= { canonical: null, examples: [] })
      const title = docs.title?.trim()
      if (docs.canonical === true) {
        set.canonical = caseName
      } else if (docs.previewable === true && title) {
        set.examples.push({ case: caseName, title })
      }
    }

    const out: RuleFixturesData = {}
    for (const [rule, set] of Object.entries(byRule)) {
      if (set.canonical === null) continue
      out[rule] = {
        canonical : set.canonical,
        examples  : set.examples.sort((a, b) => sortKey(a.title).localeCompare(sortKey(b.title)))
      }
    }
    return out
  }
})
