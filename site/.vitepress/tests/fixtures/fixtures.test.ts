import fs   from 'node:fs'
import path from 'node:path'

import * as entry                              from '../../lib/fixtures/entry'
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
    expect(entry.fixtureId('align_equals', 'basic_run')).toBe('align_equals/basic_run')
  })

  it('keys every walked case the same way', () => {
    expect(CASES.every(found => found.id === entry.fixtureId(found.rule, found.caseName))).toBe(true)
  })
})

describe('fixtureEntry', () => {
  const record = {
    changesSource : true,
    hasFindings   : false,
    hasToggle     : true,
    inputHtml     : '<pre>a</pre>',
    outputHtml    : '<pre>b</pre>'
  }
  const frontmatter = { fixtures: { 'align_equals/basic_run': record } }

  it('reads the entry a page carries for a rule and case', () => {
    expect(entry.fixtureEntry(frontmatter, 'align_equals', 'basic_run')).toBe(record)
  })

  it('names a case the page does not carry', () => {
    expect(() => entry.fixtureEntry(frontmatter, 'align_equals', 'absent'))
      .toThrow(/Fixture case "align_equals\/absent" not registered/)
  })

  it('names the case where the page carries no fixtures at all', () => {
    expect(() => entry.fixtureEntry({}, 'align_equals', 'basic_run'))
      .toThrow(/Fixture case "align_equals\/basic_run" not registered/)
  })
})

describe('fixtureGauge', () => {
  const gauge       = { default: 16, outputs: ['<pre>a</pre>'], setting: 'max-shift', steps: [0] }
  const frontmatter = { fixtureGauges: { [entry.gaugeId('align_equals', 'wide', 'max-shift')]: gauge } }

  it('reads the gauge a page carries for a case and the setting it steps', () => {
    expect(entry.fixtureGauge(frontmatter, 'align_equals', 'wide', 'max-shift')).toBe(gauge)
  })

  it('names a gauge the page does not carry', () => {
    expect(() => entry.fixtureGauge(frontmatter, 'align_equals', 'wide', 'enabled'))
      .toThrow(/Fixture gauge "align_equals\/wide#enabled" not registered/)
  })
})

describe('fixtureTrail', () => {
  const trail       = { outputs: ['<pre>a</pre>'], rules: ['align-equals'], steps: [0, 0] }
  const frontmatter = { fixtureTrails: { [entry.fixtureId(entry.COMPOSITION_RULE, 'alpha_case')]: trail } }

  it('reads the trail a page carries for a composition case', () => {
    expect(entry.fixtureTrail(frontmatter, 'alpha_case')).toBe(trail)
  })

  it('names a trail the page does not carry', () => {
    expect(() => entry.fixtureTrail({}, 'alpha_case')).toThrow(/Composition trail ".*alpha_case" not registered/)
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
    expect(names).toStrictEqual([...names].sort())
  })
})

describe('readFixtureToggle', () => {
  it('derives toggle state from an input and snapshot pair', async () => {
    const withSnap = CASES.find(c => fs.existsSync(walker.snapshotPath(c.inputPath)))!
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
    expect(readLintFindings(absent)).toStrictEqual([])
  })
})

describe('overrideToml', () => {
  it('drops the harness table and keeps every other override', () => {
    expect(walker.overrideToml({ 'code-line-length': 60, harness: { rules: ['align-equals'] } }))
      .toBe('code-line-length = 60\n')
  })

  it('renders a nested override table', () => {
    expect(walker.overrideToml({ imports: { 'first-party': ['myapp'] } }))
      .toBe('[imports]\nfirst-party = [ "myapp" ]\n')
  })

  it('returns an empty string where nothing survives the harness lift', () => {
    expect(walker.overrideToml({ harness: { rules: [] } })).toBe('')
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
      .toStrictEqual(['canonical', 'description', 'previewable', 'sandbox', 'title'])
  })

  it('returns undefined when meta.toml is absent', () => {
    expect(walker.readFixtureDocs(absent)).toBeUndefined()
  })
})

describe('readOverrideToml', () => {
  it('reads the override a case carries beside its input', () => {
    const configured = CASES.find(found => hasSidecar(found.inputPath, walker.CONFIG_FILE)
      && walker.readOverrideToml(found.inputPath) !== '')!
    expect(walker.readOverrideToml(configured.inputPath)).toMatch(/=/)
  })

  it('returns an empty string where a case carries no config', () => {
    expect(walker.readOverrideToml(absent)).toBe('')
  })
})
