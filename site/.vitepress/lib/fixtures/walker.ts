import fs   from 'node:fs'
import path from 'node:path'

export const FIXTURES_DIR  = 'tests/fixtures'
export const INPUT_SUFFIX  = '.input.py'
export const SNAPSHOTS_DIR = 'tests/snapshots'

export interface FixtureWalkEntry {
  caseName  : string
  inputPath : string
  rule      : string
}

export function* walkFixtures(repoRoot: string): Generator<FixtureWalkEntry> {
  const fixturesRoot = path.join(repoRoot, FIXTURES_DIR)
  const ruleDirs     = fs.readdirSync(fixturesRoot, { withFileTypes: true })
    .filter(d => d.isDirectory())
    .map(d => d.name)
    .sort()
  for (const rule of ruleDirs) {
    const ruleDir = path.join(fixturesRoot, rule)
    for (const file of fs.readdirSync(ruleDir).sort()) {
      if (!file.endsWith(INPUT_SUFFIX)) continue
      yield {
        caseName : file.slice(0, -INPUT_SUFFIX.length),
        inputPath: path.join(ruleDir, file),
        rule
      }
    }
  }
}
