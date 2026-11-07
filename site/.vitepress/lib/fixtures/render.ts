import { existsSync } from 'node:fs'
import path           from 'node:path'

import { fixtureId }                                                 from './entry'
import type { FixtureEntry, FixtureGauge, FixtureMap, FixtureTrail } from './entry'
import type { GaugeRequest }                                         from './page-cases'
import { readFixtureToggle }                                         from './toggle'
import * as variants                                                 from './variants'
import * as walker                                                   from './walker'
import { blockNodes }                                                from '../markdown/inline-nodes'
import { lintFenceMeta }                                             from '../markdown/lint-decorations'
import * as renderer                                                 from '../markdown/renderer'
import { inlineCode }                                                from '../shared/inline-code'
import { toTitleCase }                                               from '../shared/title-case'

const entries = new Map<string, Promise<FixtureEntry>>()
const gauges  = new Map<string, Promise<FixtureGauge>>()
const indexes = new Map<string, Map<string, string>>()
const trails  = new Map<string, Promise<FixtureTrail>>()

// Maps each fixture id to its input file, taking only the cases that carry both
// an input and a snapshot.
function buildCaseIndex(crate: string): Map<string, string> {
  return new Map(
    walker.walkFixtures(crate)
      .filter(({ inputPath }) => existsSync(walker.snapshotPath(inputPath)))
      .map(({ id, inputPath }) => [id, inputPath] as const)
  )
}

async function renderEntry(id: string, inputPath: string): Promise<FixtureEntry> {
  const md          = await renderer.getRenderer()
  const docs        = walker.readFixtureDocs(inputPath)
  const description = docs?.description?.trim()
  const configToml  = walker.readOverrideToml(inputPath)
  const title       = walker.fixtureTitle(docs) ?? toTitleCase(path.basename(path.dirname(inputPath)))

  const { inputRaw, output, ...flags } = await readFixtureToggle(inputPath)
  const fenceMeta = flags.hasFindings ? lintFenceMeta(id) : ''
  return {
    ...flags,
    configHtml       : configToml ? await renderer.renderFencedHtml(md, configToml, 'toml') : undefined,
    descriptionNodes : description ? blockNodes(md, description) : undefined,
    inputHtml        : await renderer.renderFencedHtml(md, inputRaw, 'python'),
    outputHtml       : await renderer.renderFencedHtml(md, output, 'python', fenceMeta),
    titleHtml        : inlineCode(title)
  }
}

// Reads the input path of one case, failing where the case has no input and
// snapshot pair.
function casePath(crate: string, id: string): string {
  const inputPath = indexes.getOrInsertComputed(crate, buildCaseIndex).get(id)
  if (inputPath === undefined) throw new Error(`Fixture case "${id}" has no input and snapshot pair`)
  return inputPath
}

// Renders the fixture cases one page shows, caching each by its input path so a
// case appearing on several pages costs one render across the whole build.
export async function fixtureEntries(crate: string, ids: readonly string[]): Promise<FixtureMap> {
  const pairs = await Promise.all(ids.map(async id => {
    const inputPath = casePath(crate, id)
    const entry     = await entries.getOrInsertComputed(inputPath, () => renderEntry(id, inputPath))
    return [id, entry] as const
  }))
  return Object.fromEntries(pairs)
}

// Steps each gauge one page draws, caching each by its id across the build.
export async function fixtureGauges(
  crate    : string,
  requests : readonly GaugeRequest[]
): Promise<Record<string, FixtureGauge>> {
  const md = await renderer.getRenderer()
  return readCached(gauges, requests.map(({ case: caseName, id, rule, setting }) =>
    [id, () => variants.readGauge(md, rule, casePath(crate, fixtureId(rule, caseName)), setting)]))
}

// Runs each composition trail one page draws, caching each by its case id
// across the build.
export async function fixtureTrails(
  crate : string,
  ids   : readonly string[]
): Promise<Record<string, FixtureTrail>> {
  const md = await renderer.getRenderer()
  return readCached(trails, ids.map(id => [id, () => variants.readTrail(md, casePath(crate, id))]))
}

// Runs each read whose id `cache` does not yet hold, keeping its result there
// for the rest of the build, and returns every result by its id.
async function readCached<T>(
  cache : Map<string, Promise<T>>,
  reads : readonly (readonly [id: string, read: () => Promise<T>])[]
): Promise<Record<string, T>> {
  return Object.fromEntries(await Promise.all(reads.map(async ([id, read]) =>
    [id, await cache.getOrInsertComputed(id, read)] as const)))
}

// Empties the index and the rendered entries, which the dev server does after
// an edit under the fixture tree so the next render reads the new contents.
export function resetFixtureEntries(): void {
  entries.clear()
  gauges.clear()
  indexes.clear()
  trails.clear()
}
