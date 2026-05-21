import fs                 from 'node:fs/promises'
import { existsSync }     from 'node:fs'
import path               from 'node:path'

import matter                      from 'gray-matter'
import { getSingletonHighlighter } from 'shiki'
import { defineLoader }            from 'vitepress'

import { FIXTURES_DIR, INPUT_SUFFIX, SNAPSHOTS_DIR, walkFixtures } from '../lib/fixtures/walker'
import { SHIKI_THEMES } from '../lib/shared/constants'
import { repoRoot }     from '../lib/shared/paths'

const root          = repoRoot(import.meta.url)
const fixturesRoot  = path.join(root, FIXTURES_DIR)
const snapshotsRoot = path.join(root, SNAPSHOTS_DIR)

export interface FixtureEntry {
  changesSource : boolean
  input         : string
  inputHtml     : string
  output        : string
  outputHtml    : string
}

export type FixtureData = Record<string, Record<string, FixtureEntry>>

declare const data: FixtureData
export { data }

export default defineLoader({
  watch: [
    `${fixturesRoot}/**/*${INPUT_SUFFIX}`,
    `${snapshotsRoot}/**/*${INPUT_SUFFIX}.snap`,
    path.join(import.meta.dirname, '../lib/shared/constants.ts')
  ],
  async load(): Promise<FixtureData> {
    const entries = [...walkFixtures(root)].filter(({ rule, caseName }) =>
      existsSync(path.join(snapshotsRoot, rule, `${caseName}${INPUT_SUFFIX}.snap`))
    )
    const rows = await Promise.all(entries.map(async ({ rule, caseName, inputPath }) => {
      const snapPath = path.join(snapshotsRoot, rule, `${caseName}${INPUT_SUFFIX}.snap`)
      const [inputRaw, snapRaw]      = await Promise.all([fs.readFile(inputPath, 'utf8'), fs.readFile(snapPath, 'utf8')])
      const output                   = matter(snapRaw).content.replace(/\s+$/, '\n')
      const [inputHtml, outputHtml]  = await Promise.all([highlight(inputRaw), highlight(output)])
      return { caseName, entry: { changesSource: inputRaw !== output, input: inputRaw, inputHtml, output, outputHtml }, rule }
    }))
    const out: FixtureData = {}
    for (const { caseName, entry, rule } of rows) (out[rule] ??= {})[caseName] = entry
    return out
  }
})

const highlighter = getSingletonHighlighter({
  langs : ['python'],
  themes: Object.values(SHIKI_THEMES)
})

async function highlight(code: string): Promise<string> {
  const h = await highlighter
  return h.codeToHtml(code, {
    lang  : 'python',
    themes: SHIKI_THEMES
  })
}

