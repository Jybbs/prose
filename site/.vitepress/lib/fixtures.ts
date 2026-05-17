import fs   from 'node:fs'
import path from 'node:path'

export const INPUT_SUFFIX   = '.input.py'
export const FIXTURES_DIR   = 'tests/fixtures'
export const SNAPSHOTS_DIR  = 'tests/snapshots'

export interface FixtureEntry {
  caseName  : string
  inputPath : string
  rule      : string
}

export function* walkFixtures(repoRoot: string): Generator<FixtureEntry> {
  const fixturesRoot = path.join(repoRoot, FIXTURES_DIR)
  for (const rule of fs.readdirSync(fixturesRoot).sort()) {
    const ruleDir = path.join(fixturesRoot, rule)
    if (!fs.statSync(ruleDir).isDirectory()) continue
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
