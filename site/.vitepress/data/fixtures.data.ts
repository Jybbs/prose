import fs   from 'node:fs'
import path from 'node:path'

import { codeToHtml }   from 'shiki'
import { defineLoader } from 'vitepress'

import { repoRoot } from '../lib/paths'

const root          = repoRoot(import.meta.url)
const fixturesRoot  = path.join(root, 'tests/fixtures')
const snapshotsRoot = path.join(root, 'tests/snapshots')
const INPUT_SUFFIX  = '.input.py'

export interface FixtureEntry {
  input      : string
  inputHtml  : string
  output     : string
  outputHtml : string
}

export type FixtureData = Record<string, Record<string, FixtureEntry>>

declare const data: FixtureData
export { data }

export default defineLoader({
  watch: [
    `${fixturesRoot}/**/*${INPUT_SUFFIX}`,
    `${snapshotsRoot}/**/*${INPUT_SUFFIX}.snap`
  ],
  async load(): Promise<FixtureData> {
    const out: FixtureData = {}
    for (const rule of fs.readdirSync(fixturesRoot).sort()) {
      const ruleDir = path.join(fixturesRoot, rule)
      if (!fs.statSync(ruleDir).isDirectory()) continue
      for (const file of fs.readdirSync(ruleDir).sort()) {
        if (!file.endsWith(INPUT_SUFFIX)) continue
        const caseName = file.slice(0, -INPUT_SUFFIX.length)
        const snapPath = path.join(snapshotsRoot, rule, `${caseName}${INPUT_SUFFIX}.snap`)
        if (!fs.existsSync(snapPath)) continue
        const input  = fs.readFileSync(path.join(ruleDir, file), 'utf8')
        const output = stripInstaHeader(fs.readFileSync(snapPath, 'utf8'))
        const [inputHtml, outputHtml] = await Promise.all([
          highlight(input),
          highlight(output)
        ])
        out[rule] ??= {}
        out[rule][caseName] = { input, inputHtml, output, outputHtml }
      }
    }
    return out
  }
})

async function highlight(code: string): Promise<string> {
  return codeToHtml(code, {
    lang  : 'python',
    themes: { light: 'github-light', dark: 'github-dark' }
  })
}

function stripInstaHeader(snap: string): string {
  return snap
    .replace(/^---\n[\s\S]*?\n---\n+/, '')
    .replace(/\s+$/, '\n')
}
