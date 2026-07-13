import { existsSync } from 'node:fs'

import { defineLoader }          from 'vitepress'
import type { MarkdownRenderer } from 'vitepress'

import { readFixtureToggle } from './toggle'
import * as walker           from './walker'
import { blockNodes, type InlineNode } from '../markdown/inline-nodes'
import { lintFenceMeta }     from '../markdown/lint-decorations'
import * as renderer         from '../markdown/renderer'
import { crateDir }          from '../shared/paths'

const crate = crateDir(import.meta.url)

interface FixtureEntry {
  changesSource    : boolean
  descriptionNodes ?: InlineNode[]
  hasFindings      : boolean
  hasToggle        : boolean
  inputHtml        : string
  outputHtml       : string
}

type FixtureData = Record<string, Record<string, FixtureEntry>>

declare const data: FixtureData
export { data }

function descriptionNodes(md: MarkdownRenderer, inputPath: string): InlineNode[] | undefined {
  const text = walker.readFixtureDocs(inputPath)?.description?.trim()
  return text ? blockNodes(md, text) : undefined
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
          descriptionNodes : descriptionNodes(md, inputPath),
          hasFindings,
          hasToggle,
          inputHtml       : await renderer.renderFencedHtml(md, inputRaw, 'python'),
          outputHtml      : await renderer.renderFencedHtml(md, output, 'python', hasFindings ? lintFenceMeta(id) : '')
        },
        rule
      }
    }))
    const out: FixtureData = {}
    for (const { caseName, entry, rule } of rows) (out[rule] ??= {})[caseName] = entry
    return out
  }
})
