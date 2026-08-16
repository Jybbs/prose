import { casesForRule, fixturesForRule } from '../../lib/rules/rule-view'

vi.mock('../../lib/rules/composition.data', () => ({
  data: { byRule: { 'align-equals': ['alpha_case', 'beta_case'] } }
}))

vi.mock('../../lib/rules/rule-fixtures.data', () => ({
  data: {
    align_equals: { canonical: 'basic_run', examples: [{ case: 'nested', title: 'Nested' }] }
  }
}))

describe('casesForRule', () => {
  it('resolves a fixture directory name to its kebab-case slug', () => {
    expect(casesForRule('align_equals')).toEqual(['alpha_case', 'beta_case'])
  })

  it('yields nothing for a rule no previewable case activates', () => {
    expect(casesForRule('wrap_docstrings')).toEqual([])
  })
})

describe('fixturesForRule', () => {
  it('returns the canonical case and examples registered for a rule', () => {
    expect(fixturesForRule('align_equals')).toEqual({
      canonical : 'basic_run',
      examples  : [{ case: 'nested', title: 'Nested' }]
    })
  })

  it('names the unregistered rule when the lookup misses', () => {
    expect(() => fixturesForRule('align-equals')).toThrow(/Rule "align-equals" not registered/)
  })
})
