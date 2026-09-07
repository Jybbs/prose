import { pageCaseIds, type PageFixtureSets } from '../../lib/fixtures/page-cases'

const SETS: PageFixtureSets = {
  composition: {
    byRule : { 'align-equals': ['alpha_case'], 'wrap-docstrings': ['gamma_case'] },
    cases  : [{ case: 'alpha_case' }, { case: 'gamma_case' }]
  },

  ruleFixtures: {
    align_equals    : { canonical: 'basic_run', examples: [{ case: 'nested' }, { case: 'deep' }] },
    strip_stranding : { canonical: 'only_case', examples: [] }
  }
}

const ids = (source: string): string[] => pageCaseIds(source, SETS)

describe('pageCaseIds', () => {
  it('reads the rule and case a Fixture tag names', () => {
    expect(ids('<Fixture rule="align_comments" case="run_shares" />'))
      .toEqual(['align_comments/run_shares'])
  })

  it('reads a FixtureConvergence tag the same way', () => {
    expect(ids('<FixtureConvergence rule="band_constants" case="stacked" />'))
      .toEqual(['band_constants/stacked'])
  })

  it('takes the canonical case, the examples, and the composition cases of a RuleLayout', () => {
    expect(ids('<RuleLayout rule="align_equals">')).toEqual([
      'align_equals/basic_run',
      'align_equals/nested',
      'align_equals/deep',
      'composition/alpha_case'
    ])
  })

  it('takes the canonical case alone where a rule joins no composition case', () => {
    expect(ids('<RuleLayout rule="strip_stranding">')).toEqual(['strip_stranding/only_case'])
  })

  it('takes every previewable case where a bare CompositionCards narrows nothing', () => {
    expect(ids('<CompositionCards />'))
      .toEqual(['composition/alpha_case', 'composition/gamma_case'])
  })

  it.each([
    ['a RuleLayout naming an unregistered rule', '<RuleLayout rule="not_a_rule">'],
    ['a RuleLayout naming no rule',              '<RuleLayout>'],
    ['a Fixture missing its case',               '<Fixture rule="align_equals" />'],
    ['a Fixture missing its rule',               '<Fixture case="basic_run" />'],
    ['a page carrying no fixture tag',           '# Heading\n\nPlain prose.\n'],
    ['a closing tag alone',                      '</RuleLayout>']
  ])('reaches no case from %s', (_label, source) => {
    expect(ids(source)).toEqual([])
  })

  it('reads attributes in either order', () => {
    expect(ids('<Fixture case="run_shares" rule="align_comments" />'))
      .toEqual(['align_comments/run_shares'])
  })

  it('reports a case once where a page names it twice', () => {
    expect(ids('<Fixture rule="r" case="c" />\n<Fixture rule="r" case="c" />')).toEqual(['r/c'])
  })

  it('collects every tag on a page in source order', () => {
    const source = [
      '<RuleLayout rule="strip_stranding">',
      '<Fixture rule="composition" case="hoisted" />',
      '</RuleLayout>'
    ].join('\n')
    expect(ids(source)).toEqual(['strip_stranding/only_case', 'composition/hoisted'])
  })
})
