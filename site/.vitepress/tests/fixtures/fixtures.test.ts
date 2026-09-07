import fs   from 'node:fs'
import path from 'node:path'

import { fixtureEntry, fixtureId }              from '../../lib/fixtures/entry'
import { LINT_FINDINGS_FILE, readLintFindings } from '../../lib/fixtures/lint-findings'
import { readFixtureToggle }                    from '../../lib/fixtures/toggle'
import * as walker                              from '../../lib/fixtures/walker'
import { CASES, CRATE, hasSidecar }             from '../corpus'

const absent = path.join(CRATE, 'tests', 'fixtures', '__no_such_case__', 'input.py')

describe('walkFixtures', () => {
  it('yields a rule/case/input entry per fixture case', () => {
    expect(CASES.length).toBeGreaterThan(0)
    expect(CASES[0].inputPath.endsWith('input.py')).toBe(true)
    expect(CASES[0].rule).toBeTruthy()
    expect(CASES[0].caseName).toBeTruthy()
    expect(CASES[0].id).toBe(`${CASES[0].rule}/${CASES[0].caseName}`)
  })
})

describe('fixtureId', () => {
  it('joins a rule and case into the key a page frontmatter carries', () => {
    expect(fixtureId('align_equals', 'basic_run')).toBe('align_equals/basic_run')
  })

  it('keys every walked case the same way', () => {
    expect(CASES.every(entry => entry.id === fixtureId(entry.rule, entry.caseName))).toBe(true)
  })
})

describe('fixtureEntry', () => {
  const entry = {
    changesSource : true,
    hasFindings   : false,
    hasToggle     : true,
    inputHtml     : '<pre>a</pre>',
    outputHtml    : '<pre>b</pre>'
  }
  const frontmatter = { fixtures: { 'align_equals/basic_run': entry } }

  it('reads the entry a page carries for a rule and case', () => {
    expect(fixtureEntry(frontmatter, 'align_equals', 'basic_run')).toBe(entry)
  })

  it('names a case the page does not carry', () => {
    expect(() => fixtureEntry(frontmatter, 'align_equals', 'absent'))
      .toThrow(/Fixture case "align_equals\/absent" not registered/)
  })

  it('names the case where the page carries no fixtures at all', () => {
    expect(() => fixtureEntry({}, 'align_equals', 'basic_run'))
      .toThrow(/Fixture case "align_equals\/basic_run" not registered/)
  })
})

describe('corpusLintFindings', () => {
  it('maps every findings-bearing case by its fixture id', () => {
    const map = walker.corpusLintFindings(CRATE)
    expect(map.size).toBeGreaterThan(0)
    expect([...map.keys()].every(id => {
      const parts = id.split('/')
      return parts.length === 2 && parts.every(Boolean)
    })).toBe(true)
    expect([...map.values()].every(findings => findings.length > 0)).toBe(true)
  })
})

describe('fixtureTitle', () => {
  it.each([
    [{ title: 'A Case' },     'A Case'],
    [{ title: '  A Case  ' }, 'A Case'],
    [{ title: '   ' },        undefined],
    [{ title: '' },           undefined],
    [{},                      undefined],
    [undefined,               undefined]
  ])('resolves %o to %o', (docs, expected) => {
    expect(walker.fixtureTitle(docs)).toBe(expected)
  })
})

describe('fixtureWatchGlobs', () => {
  it('covers every watched sidecar, rooted at the fixture tree', () => {
    const globs   = walker.fixtureWatchGlobs(CRATE)
    const watched = [
      'input.py', 'input.py.snap', 'config.toml', 'lint_findings.snap', 'meta.toml'
    ]
    expect(globs.every(g => g.includes('tests/fixtures'))).toBe(true)
    for (const file of watched) {
      expect(globs.some(g => g.endsWith(`/${file}`))).toBe(true)
    }
  })
})

describe('subdirNames', () => {
  it('lists rule directories in sorted order', () => {
    const names = walker.subdirNames(path.join(CRATE, 'tests', 'fixtures'))
    expect(names.length).toBeGreaterThan(0)
    expect(names).toEqual([...names].sort())
  })
})

describe('readFixtureToggle', () => {
  it('derives toggle state from an input and snapshot pair', async () => {
    const withSnap = CASES.find(c => fs.existsSync(`${c.inputPath}.snap`))!
    const state    = await readFixtureToggle(withSnap.inputPath)
    expect(state.inputRaw.length).toBeGreaterThan(0)
    expect(state.hasToggle).toBe(state.changesSource || state.hasFindings)
  })
})

describe('readLintFindings', () => {
  it('parses the sidecar when a case carries one', () => {
    const withSidecar = CASES.find(c => hasSidecar(c.inputPath, LINT_FINDINGS_FILE))!
    expect(readLintFindings(withSidecar.inputPath).length).toBeGreaterThan(0)
  })

  it('returns an empty list when no sidecar is present', () => {
    expect(readLintFindings(absent)).toEqual([])
  })
})

describe('readFixtureDocs', () => {
  it('reads the [docs] table from meta.toml when present', () => {
    const withMeta = CASES.find(c => hasSidecar(c.inputPath, 'meta.toml'))!
    expect(Object.keys(walker.readFixtureDocs(withMeta.inputPath)!).length).toBeGreaterThan(0)
  })

  it('reads the documented key set and no other, across every case', () => {
    const keys = new Set(CASES.flatMap(c => Object.keys(walker.readFixtureDocs(c.inputPath) ?? {})))
    expect([...keys].sort())
      .toEqual(['canonical', 'description', 'previewable', 'sandbox', 'title'])
  })

  it('returns undefined when meta.toml is absent', () => {
    expect(walker.readFixtureDocs(absent)).toBeUndefined()
  })
})
