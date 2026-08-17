import * as composition from '../../lib/rules/composition'
import { fixtureDir }   from '../support'

const fixture = (name: string): string => fixtureDir(import.meta.dirname, name)

const cases = (name = 'composition'): composition.CompositionCase[] =>
  composition.readCompositionCases(fixture(name))

describe('readCompositionCases', () => {
  it('reads the cases a meta.toml marks previewable and skips the rest', () => {
    expect(cases().map(entry => entry.case)).toEqual(['alpha_case', 'bare_title_case', 'beta_case'])
  })

  it('titles each case from its meta.toml, falling back to the directory name', () => {
    expect(cases().map(entry => entry.title))
      .toEqual(['Alpha Runs Ahead of Beta', 'Bare Title Case', 'Beta Settles Alone'])
  })

  it('carries each case source verbatim', () => {
    expect(cases()[2].source).toBe('gamma = 3\n')
  })

  it('seeds the sidecar config with the harness table lifted out', () => {
    expect(cases()[0].configToml).toBe('code-line-length = 60\n')
  })

  it('seeds an empty config where the sidecar carries harness keys alone', () => {
    expect(cases()[2].configToml).toBe('')
  })

  it('carries the harness rule list each case activates', () => {
    expect(cases()[0].rules).toEqual(['align-equals', 'space-statements'])
  })

  it('rejects a case declaring no harness rules even where it never renders', () => {
    expect(() => cases('composition-no-rules')).toThrow(/missing \[harness\]\.rules/)
  })
})

describe('byRule', () => {
  it('inverts the per-case rule lists into the cases each rule takes part in', () => {
    expect(composition.byRule(cases())).toEqual({
      'align-equals'     : ['alpha_case', 'beta_case'],
      'space-statements' : ['alpha_case', 'bare_title_case']
    })
  })

  it('indexes nothing when no case is previewable', () => {
    expect(composition.byRule([])).toEqual({})
  })
})

describe('seedToml', () => {
  it('drops the harness table and keeps every other override', () => {
    expect(composition.seedToml({ 'code-line-length': 60, harness: { rules: ['align-equals'] } }))
      .toBe('code-line-length = 60\n')
  })

  it('renders a nested override table', () => {
    expect(composition.seedToml({ imports: { 'first-party': ['myapp'] } }))
      .toBe('[imports]\nfirst-party = [ "myapp" ]\n')
  })

  it('returns an empty string where nothing survives the harness lift', () => {
    expect(composition.seedToml({ harness: { rules: [] } })).toBe('')
  })
})
