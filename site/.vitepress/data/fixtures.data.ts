import { existsSync } from 'node:fs'
import fs             from 'node:fs/promises'
import path          from 'node:path'

import matter           from 'gray-matter'
import { defineLoader } from 'vitepress'

import { FIXTURES_DIR, INPUT_FILE, SNAPSHOT_FILE, walkFixtures } from '../lib/fixtures/walker'
import { getRenderer, renderFencedHtml } from '../lib/markdown/renderer'
import { repoRoot }                      from '../lib/shared/paths'

const root         = repoRoot(import.meta.url)
const fixturesRoot = path.join(root, FIXTURES_DIR)

interface FixtureEntry {
  changesSource : boolean
  input         : string
  inputHtml     : string
  output        : string
  outputHtml    : string
}

type FixtureData = Record<string, Record<string, FixtureEntry>>

declare const data: FixtureData
export { data }

export default defineLoader({
  watch: [
    `${fixturesRoot}/**/${INPUT_FILE}`,
    `${fixturesRoot}/**/${SNAPSHOT_FILE}`
  ],
  async load(): Promise<FixtureData> {
    const md      = await getRenderer()
    const entries = [...walkFixtures(root)].filter(({ inputPath }) => existsSync(`${inputPath}.snap`))
    const rows = await Promise.all(entries.map(async ({ rule, caseName, inputPath }) => {
      const [inputRaw, snapRaw] = await Promise.all([
        fs.readFile(inputPath,           'utf8'),
        fs.readFile(`${inputPath}.snap`, 'utf8')
      ])
      const output              = matter(snapRaw).content.replace(/\s+$/, '\n')
      return {
        caseName,
        entry: {
          changesSource : inputRaw !== output,
          input         : inputRaw,
          inputHtml     : renderFencedHtml(md, inputRaw, 'python'),
          output,
          outputHtml    : renderFencedHtml(md, output, 'python')
        },
        rule
      }
    }))
    const out: FixtureData = {}
    for (const { caseName, entry, rule } of rows) (out[rule] ??= {})[caseName] = entry
    return out
  }
})
