import fs   from 'node:fs'
import path from 'node:path'

import { codeToHtml }   from 'shiki'
import { defineLoader } from 'vitepress'

import { SHIKI_THEMES } from '../lib/constants'
import { FIXTURES_DIR, INPUT_SUFFIX, SNAPSHOTS_DIR, walkFixtures } from '../lib/fixtures'
import { repoRoot }     from '../lib/paths'
import type { Registry } from '../lib/types'

const root          = repoRoot(import.meta.url)
const fixturesRoot  = path.join(root, FIXTURES_DIR)
const snapshotsRoot = path.join(root, SNAPSHOTS_DIR)

export interface FixtureEntry {
  input      : string
  inputHtml  : string
  output     : string
  outputHtml : string
}

export type FixtureData = Registry<Registry<FixtureEntry>>

declare const data: FixtureData
export { data }

export default defineLoader({
  watch: [
    `${fixturesRoot}/**/*${INPUT_SUFFIX}`,
    `${snapshotsRoot}/**/*${INPUT_SUFFIX}.snap`
  ],
  async load(): Promise<FixtureData> {
    const out: FixtureData = {}
    for (const { rule, caseName, inputPath } of walkFixtures(root)) {
      const snapPath = path.join(snapshotsRoot, rule, `${caseName}${INPUT_SUFFIX}.snap`)
      if (!fs.existsSync(snapPath)) continue
      const input  = fs.readFileSync(inputPath, 'utf8')
      const output = stripInstaHeader(fs.readFileSync(snapPath, 'utf8'))
      const [inputHtml, outputHtml] = await Promise.all([
        highlight(input),
        highlight(output)
      ])
      out[rule] ??= {}
      out[rule][caseName] = { input, inputHtml, output, outputHtml }
    }
    return out
  }
})

async function highlight(code: string): Promise<string> {
  return codeToHtml(code, {
    lang  : 'python',
    themes: SHIKI_THEMES
  })
}

function stripInstaHeader(snap: string): string {
  return snap
    .replace(/^---\n[\s\S]*?\n---\n+/, '')
    .replace(/\s+$/, '\n')
}
