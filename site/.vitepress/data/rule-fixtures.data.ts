import { existsSync } from 'node:fs'
import fs             from 'node:fs/promises'
import path          from 'node:path'

import { parse }        from 'smol-toml'
import { defineLoader } from 'vitepress'

import { FIXTURES_DIR, walkFixtures } from '../lib/fixtures/walker'
import { repoRoot }                   from '../lib/shared/paths'

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
  watch: [`${fixturesDir}/*/*/meta.toml`],
  async load(): Promise<RuleFixturesData> {
    type Docs    = { canonical?: boolean, title?: string }
    type Pending = { canonical: string | null, examples: RuleExample[] }

    const byRule: Record<string, Pending> = {}
    for (const { rule, caseName, inputPath } of walkFixtures(root)) {
      const metaPath = path.join(path.dirname(inputPath), 'meta.toml')
      if (!existsSync(metaPath)) continue
      const docs = (parse(await fs.readFile(metaPath, 'utf8')) as { docs?: Docs }).docs
      if (docs === undefined) {
        throw new Error(`rule-fixtures.data: ${rule}/${caseName}/meta.toml missing [docs]`)
      }
      const set = (byRule[rule] ??= { canonical: null, examples: [] })
      if (docs.canonical === true) {
        set.canonical = caseName
      } else if (typeof docs.title === 'string' && docs.title.length > 0) {
        set.examples.push({ case: caseName, title: docs.title })
      }
    }

    const out: RuleFixturesData = {}
    for (const [rule, set] of Object.entries(byRule)) {
      if (set.canonical === null) {
        throw new Error(`rule-fixtures.data: rule "${rule}" has no canonical = true case`)
      }
      out[rule] = {
        canonical : set.canonical,
        examples  : set.examples.sort((a, b) => sortKey(a.title).localeCompare(sortKey(b.title)))
      }
    }
    return out
  }
})
