import fs   from 'node:fs'
import path from 'node:path'

import { fixturesDirFrom } from '../shared/paths'
import { parseToml }       from '../shared/toml'
import * as lintFindings   from './lint-findings'

export const CONFIG_FILE = 'config.toml'
export const INPUT_FILE  = 'input.py'

const META_FILE     = 'meta.toml'
const SNAPSHOT_FILE = snapshotPath(INPUT_FILE)

interface FixtureDocs {
  canonical   ?: boolean
  description ?: string
  previewable ?: boolean
  sandbox     ?: boolean
  title       ?: string
}

interface FixtureWalkEntry {
  caseName  : string
  id        : string
  inputPath : string
  rule      : string
}

export function corpusLintFindings(crateDir: string): Map<string, lintFindings.LintFinding[]> {
  const map = new Map<string, lintFindings.LintFinding[]>()
  for (const { id, inputPath } of walkFixtures(crateDir)) {
    const findings = lintFindings.readLintFindings(inputPath)
    if (findings.length > 0) map.set(id, findings)
  }
  return map
}

// The authored title a `meta.toml` carries, blank and whitespace-only alike
// resolving to nothing so each caller applies its own fallback.
export function fixtureTitle(docs: FixtureDocs | undefined): string | undefined {
  return docs?.title?.trim() || undefined
}

export function fixtureWatchGlobs(crateDir: string): string[] {
  const fixturesRoot = fixturesDirFrom(crateDir)
  return [
    `${fixturesRoot}/**/${INPUT_FILE}`,
    `${fixturesRoot}/**/${SNAPSHOT_FILE}`,
    `${fixturesRoot}/*/*/${CONFIG_FILE}`,
    `${fixturesRoot}/*/*/${lintFindings.LINT_FINDINGS_FILE}`,
    `${fixturesRoot}/*/*/${META_FILE}`
  ]
}

export function readFixtureDocs(inputPath: string): FixtureDocs | undefined {
  const metaPath = path.join(path.dirname(inputPath), META_FILE)
  if (!fs.existsSync(metaPath)) return undefined
  return (parseToml(metaPath) as { docs?: FixtureDocs }).docs
}

export function snapshotPath(inputPath: string): string {
  return `${inputPath}.snap`
}

export function subdirNames(dir: string): string[] {
  return fs.readdirSync(dir, { withFileTypes: true })
    .filter(d => d.isDirectory())
    .map(d => d.name)
    .sort()
}

export function* walkFixtures(crateDir: string): Generator<FixtureWalkEntry> {
  const fixturesRoot = fixturesDirFrom(crateDir)
  for (const rule of subdirNames(fixturesRoot)) {
    const ruleDir = path.join(fixturesRoot, rule)
    for (const caseName of subdirNames(ruleDir)) {
      const inputPath = path.join(ruleDir, caseName, INPUT_FILE)
      if (!fs.existsSync(inputPath)) continue
      yield { caseName, id: `${rule}/${caseName}`, inputPath, rule }
    }
  }
}
