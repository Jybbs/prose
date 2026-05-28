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

export function* walkFixtures(repoRoot: string): Generator<FixtureWalkEntry> {
  const fixturesRoot = path.join(repoRoot, FIXTURES_DIR)
  const subdirs      = (dir: string) => fs.readdirSync(dir, { withFileTypes: true })
    .filter(d => d.isDirectory())
    .map(d => d.name)
    .sort()
  for (const rule of subdirs(fixturesRoot)) {
    const ruleDir = path.join(fixturesRoot, rule)
    for (const caseName of subdirs(ruleDir)) {
      const inputPath = path.join(ruleDir, caseName, INPUT_FILE)
      if (!fs.existsSync(inputPath)) continue
      yield { caseName, inputPath, rule }
    }
  }
}
