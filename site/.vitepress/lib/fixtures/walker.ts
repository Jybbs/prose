import fs   from 'node:fs'
import path from 'node:path'

import { parse } from 'smol-toml'

import { LINT_FINDINGS_FILE } from './lint-findings'

const FIXTURES_DIR  = 'tests/fixtures'
const INPUT_FILE    = 'input.py'
const META_FILE     = 'meta.toml'
const SNAPSHOT_FILE = 'input.py.snap'

interface FixtureDocs {
  canonical   ?: boolean
  description ?: string
  previewable ?: boolean
  title       ?: string
}

interface FixtureWalkEntry {
  caseName  : string
  inputPath : string
  rule      : string
}

export function fixtureWatchGlobs(crateDir: string): string[] {
  const fixturesRoot = path.join(crateDir, FIXTURES_DIR)
  return [
    `${fixturesRoot}/**/${INPUT_FILE}`,
    `${fixturesRoot}/**/${SNAPSHOT_FILE}`,
    `${fixturesRoot}/*/*/${LINT_FINDINGS_FILE}`,
    `${fixturesRoot}/*/*/${META_FILE}`
  ]
}

export function readFixtureDocs(inputPath: string): FixtureDocs | undefined {
  const metaPath = path.join(path.dirname(inputPath), META_FILE)
  if (!fs.existsSync(metaPath)) return undefined
  return (parse(fs.readFileSync(metaPath, 'utf8')) as { docs?: FixtureDocs }).docs
}

export function subdirNames(dir: string): string[] {
  return fs.readdirSync(dir, { withFileTypes: true })
    .filter(d => d.isDirectory())
    .map(d => d.name)
    .sort()
}

export function* walkFixtures(crateDir: string): Generator<FixtureWalkEntry> {
  const fixturesRoot = path.join(crateDir, FIXTURES_DIR)
  for (const rule of subdirNames(fixturesRoot)) {
    const ruleDir = path.join(fixturesRoot, rule)
    for (const caseName of subdirNames(ruleDir)) {
      const inputPath = path.join(ruleDir, caseName, INPUT_FILE)
      if (!fs.existsSync(inputPath)) continue
      yield { caseName, inputPath, rule }
    }
  }
}
