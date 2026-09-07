import { existsSync } from 'node:fs'

import type { FixtureEntry, FixtureMap } from './entry'
import { readFixtureToggle }             from './toggle'
import * as walker                       from './walker'
import { blockNodes }                    from '../markdown/inline-nodes'
import { lintFenceMeta }                 from '../markdown/lint-decorations'
import * as renderer                     from '../markdown/renderer'

const entries = new Map<string, Promise<FixtureEntry>>()
const indexes = new Map<string, Map<string, string>>()

// Maps each fixture id to its input file, taking only the cases that carry both
// an input and a snapshot.
function buildCaseIndex(crate: string): Map<string, string> {
  return new Map(
    walker.walkFixtures(crate)
      .filter(({ inputPath }) => existsSync(walker.snapshotPath(inputPath)))
      .map(({ id, inputPath }) => [id, inputPath] as const)
  )
}

// Renders the fixture cases one page reaches, caching each by its input path so
// a case several pages render costs one render across the whole build.
export async function fixtureEntries(crate: string, ids: readonly string[]): Promise<FixtureMap> {
  const index  = indexes.getOrInsertComputed(crate, buildCaseIndex)
  const walked = await Promise.all(ids.map(async id => {
    const inputPath = index.get(id)
    if (inputPath === undefined) {
      throw new Error(`Fixture case "${id}" has no input and snapshot pair`)
    }
    const entry = await entries.getOrInsertComputed(inputPath, () => renderEntry(id, inputPath))
    return [id, entry] as const
  }))
  return Object.fromEntries(walked)
}

// Empties the index and the rendered entries, which the dev server calls after
// an edit under the fixture tree so the next render reads the new contents.
export function resetFixtureEntries(): void {
  entries.clear()
  indexes.clear()
}

async function renderEntry(id: string, inputPath: string): Promise<FixtureEntry> {
  const md          = await renderer.getRenderer()
  const description = walker.readFixtureDocs(inputPath)?.description?.trim()

  const { inputRaw, output, ...flags } = await readFixtureToggle(inputPath)
  const fenceMeta = flags.hasFindings ? lintFenceMeta(id) : ''
  return {
    ...flags,
    descriptionNodes : description ? blockNodes(md, description) : undefined,
    inputHtml        : await renderer.renderFencedHtml(md, inputRaw, 'python'),
    outputHtml       : await renderer.renderFencedHtml(md, output, 'python', fenceMeta)
  }
}
