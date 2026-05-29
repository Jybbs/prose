import fs   from 'node:fs'
import path from 'node:path'

export const FIXTURES_DIR  = 'tests/fixtures'
export const INPUT_FILE    = 'input.py'
export const SNAPSHOT_FILE = 'input.py.snap'

interface FixtureWalkEntry {
  caseName  : string
  inputPath : string
  rule      : string
}

export function subdirNames(dir: string): string[] {
  return fs.readdirSync(dir, { withFileTypes: true })
    .filter(d => d.isDirectory())
    .map(d => d.name)
    .sort()
}

export function* walkFixtures(repoRoot: string): Generator<FixtureWalkEntry> {
  const fixturesRoot = path.join(repoRoot, FIXTURES_DIR)
  for (const rule of subdirNames(fixturesRoot)) {
    const ruleDir = path.join(fixturesRoot, rule)
    for (const caseName of subdirNames(ruleDir)) {
      const inputPath = path.join(ruleDir, caseName, INPUT_FILE)
      if (!fs.existsSync(inputPath)) continue
      yield { caseName, inputPath, rule }
    }
  }
}
