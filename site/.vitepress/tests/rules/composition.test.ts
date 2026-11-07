import * as composition from '../../lib/rules/composition'
import { fixtureDir }   from '../support'

const fixture = (name: string): string => fixtureDir(import.meta.dirname, name)

const cases = (name = 'composition'): composition.CompositionCase[] =>
  composition.readCompositionCases(fixture(name))

describe('readCompositionCases', () => {
  it('reads the cases a meta.toml marks previewable and skips the rest', () => {
    expect(cases().map(entry => entry.case)).toStrictEqual(['alpha_case', 'bare_title_case', 'beta_case'])
  })

  it('carries the harness rule list each case activates', () => {
    expect(cases()[0].rules).toStrictEqual(['align-equals', 'space-statements'])
  })

  it('rejects a case declaring no harness rules even where it never renders', () => {
    expect(() => cases('composition-no-rules')).toThrow(/missing \[harness\]\.rules/)
  })
})

describe('compositionDir', () => {
  it('resolves the composition directory inside a crate fixture tree', () => {
    expect(composition.compositionDir('/repo/crate'))
      .toBe('/repo/crate/tests/fixtures/composition')
  })
})

describe('readCompositionData', () => {
  it('pairs the previewable cases with an index built from those same cases', () => {
    const data = composition.readCompositionData(fixture('composition'))
    expect(data.cases).toStrictEqual(cases())
    expect(data.byRule).toStrictEqual(composition.byRule(data.cases))
  })
})

describe('byRule', () => {
  it('inverts the per-case rule lists into the cases each rule takes part in', () => {
    expect(composition.byRule(cases())).toStrictEqual({
      'align-equals'     : ['alpha_case', 'beta_case'],
      'space-statements' : ['alpha_case', 'bare_title_case']
    })
  })

  it('indexes nothing when no case is previewable', () => {
    expect(composition.byRule([])).toStrictEqual({})
  })
})
