import { existsSync } from 'node:fs'

import { defineLoader } from 'vitepress'

import { readFixtureToggle } from '../lib/fixtures/toggle'
import * as walker           from '../lib/fixtures/walker'
import { lintFenceMeta }     from '../lib/markdown/lint-decorations'
import * as renderer         from '../lib/markdown/renderer'
import { crateDir }          from '../lib/shared/paths'

const crate = crateDir(import.meta.url)

interface FixtureEntry {
  changesSource    : boolean
  descriptionHtml ?: string
  hasFindings      : boolean
  hasToggle        : boolean
  inputHtml        : string
  outputHtml       : string
}

type FixtureData = Record<string, Record<string, FixtureEntry>>

declare const data: FixtureData
export { data }

function descriptionHtml(
  md        : Awaited<ReturnType<typeof renderer.getRenderer>>,
  inputPath : string
): string | undefined {
  const text = walker.readFixtureDocs(inputPath)?.description?.trim()
  return text ? renderer.renderBlockHtml(md, text) : undefined
}

export default defineLoader({
  watch: walker.fixtureWatchGlobs(crate),
  async load(): Promise<FixtureData> {
    const md      = await renderer.getRenderer()
    const entries = [...walker.walkFixtures(crate)]
      .filter(({ inputPath }) => existsSync(walker.snapshotPath(inputPath)))
    const rows = await Promise.all(entries.map(async ({ caseName, id, inputPath, rule }) => {
      const { changesSource, hasFindings, hasToggle, inputRaw, output } =
        await readFixtureToggle(inputPath)
      return {
        caseName,
        entry: {
          changesSource,
          descriptionHtml : descriptionHtml(md, inputPath),
          hasFindings,
          hasToggle,
          inputHtml       : renderer.renderFencedHtml(md, inputRaw, 'python'),
          outputHtml      : renderer.renderFencedHtml(md, output, 'python', hasFindings ? lintFenceMeta(id) : '')
        },
        rule
      }
    }))
    const out: FixtureData = {}
    for (const { caseName, entry, rule } of rows) (out[rule] ??= {})[caseName] = entry
    return out
  }
})
