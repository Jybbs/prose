import { existsSync } from 'node:fs'
import fs             from 'node:fs/promises'
import path          from 'node:path'

import matter           from 'gray-matter'
import { defineLoader } from 'vitepress'

import { LINT_FINDINGS_FILE, type LintFinding, lintDecorations, readLintFindings } from '../lib/fixtures/lint-findings'
import {
  FIXTURES_DIR, INPUT_FILE, META_FILE, SNAPSHOT_FILE, readFixtureDocs, walkFixtures
} from '../lib/fixtures/walker'
import { getRenderer, renderFencedHtml } from '../lib/markdown/renderer'
import { repoRoot }                      from '../lib/shared/paths'

const root         = repoRoot(import.meta.url)
const fixturesRoot = path.join(root, FIXTURES_DIR)

interface FixtureEntry {
  changesSource    : boolean
  descriptionHtml ?: string
  findings         : readonly LintFinding[]
  inputHtml        : string
  outputHtml       : string
}

type FixtureData = Record<string, Record<string, FixtureEntry>>

declare const data: FixtureData
export { data }

function descriptionHtml(
  md        : Awaited<ReturnType<typeof getRenderer>>,
  inputPath : string
): string | undefined {
  const text = readFixtureDocs(inputPath)?.description?.trim()
  return text ? md.render(text) : undefined
}

export default defineLoader({
  watch: [
    `${fixturesRoot}/**/${INPUT_FILE}`,
    `${fixturesRoot}/**/${SNAPSHOT_FILE}`,
    `${fixturesRoot}/*/*/${LINT_FINDINGS_FILE}`,
    `${fixturesRoot}/*/*/${META_FILE}`
  ],
  async load(): Promise<FixtureData> {
    const md      = await getRenderer()
    const entries = [...walkFixtures(root)].filter(({ inputPath }) => existsSync(`${inputPath}.snap`))
    const rows = await Promise.all(entries.map(async ({ rule, caseName, inputPath }) => {
      const [inputRaw, snapRaw] = await Promise.all([
        fs.readFile(inputPath,           'utf8'),
        fs.readFile(`${inputPath}.snap`, 'utf8')
      ])
      const output      = matter(snapRaw).content.replace(/\s+$/, '\n')
      const findings    = readLintFindings(inputPath)
      const decorations = lintDecorations(findings)
      return {
        caseName,
        entry: {
          changesSource   : inputRaw !== output,
          descriptionHtml : descriptionHtml(md, inputPath),
          findings,
          inputHtml       : renderFencedHtml(md, inputRaw, 'python'),
          outputHtml      : renderFencedHtml(md, output, 'python', decorations)
        },
        rule
      }
    }))
    const out: FixtureData = {}
    for (const { caseName, entry, rule } of rows) (out[rule] ??= {})[caseName] = entry
    return out
  }
})
