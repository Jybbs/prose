import fs   from 'node:fs'
import path from 'node:path'

import { LINT_FINDINGS_FILE, readLintFindings } from '../../lib/fixtures/lint-findings'
import { readFixtureToggle }                    from '../../lib/fixtures/toggle'
import {
  fixtureWatchGlobs, readFixtureDocs, subdirNames, walkFixtures
} from '../../lib/fixtures/walker'
import { repoRoot } from '../../lib/shared/paths'

const root    = repoRoot(import.meta.url)
const cases   = [...walkFixtures(root)]
const absent  = path.join(root, 'tests', 'fixtures', '__no_such_case__', 'input.py')
const sidecar = (inputPath: string, file: string): boolean =>
  fs.existsSync(path.join(path.dirname(inputPath), file))

describe('walkFixtures', () => {
  it('yields a rule/case/input entry per fixture case', () => {
    expect(cases.length).toBeGreaterThan(0)
    expect(cases[0].inputPath.endsWith('input.py')).toBe(true)
    expect(cases[0].rule).toBeTruthy()
    expect(cases[0].caseName).toBeTruthy()
  })
})

describe('fixtureWatchGlobs', () => {
  it('builds four globs rooted at the fixture tree', () => {
    const globs = fixtureWatchGlobs(root)
    expect(globs).toHaveLength(4)
    expect(globs.every(g => g.includes('tests/fixtures'))).toBe(true)
  })
})

describe('subdirNames', () => {
  it('lists rule directories in sorted order', () => {
    const names = subdirNames(path.join(root, 'tests', 'fixtures'))
    expect(names.length).toBeGreaterThan(0)
    expect(names).toEqual([...names].sort())
  })
})

describe('readFixtureToggle', () => {
  it('derives toggle state from an input and snapshot pair', async () => {
    const withSnap = cases.find(c => fs.existsSync(`${c.inputPath}.snap`))!
    const state    = await readFixtureToggle(withSnap.inputPath)
    expect(state.inputRaw.length).toBeGreaterThan(0)
    expect(state.hasToggle).toBe(state.changesSource || state.hasFindings)
  })
})

describe('readLintFindings', () => {
  it('parses the sidecar when a case carries one', () => {
    const withSidecar = cases.find(c => sidecar(c.inputPath, LINT_FINDINGS_FILE))!
    expect(readLintFindings(withSidecar.inputPath).length).toBeGreaterThan(0)
  })

  it('returns an empty list when no sidecar is present', () => {
    expect(readLintFindings(absent)).toEqual([])
  })
})

describe('readFixtureDocs', () => {
  it('reads the [docs] table from meta.toml when present', () => {
    const withMeta = cases.find(c => sidecar(c.inputPath, 'meta.toml'))!
    const docs     = readFixtureDocs(withMeta.inputPath)!
    expect(Object.keys(docs).length).toBeGreaterThan(0)
    expect(Object.keys(docs).every(k =>
      ['canonical', 'description', 'previewable', 'title'].includes(k))).toBe(true)
  })

  it('returns undefined when meta.toml is absent', () => {
    expect(readFixtureDocs(absent)).toBeUndefined()
  })
})
