import * as ruleView from '../../lib/rules/rule-view'

vi.mock('../../lib/rules/composition.data', () => ({
  data: {
    byRule : { 'align-equals': ['alpha_case', 'beta_case'] },
    cases  : [{ case: 'alpha_case' }, { case: 'beta_case' }, { case: 'gamma_case' }]
  }
}))

vi.mock('../../lib/rules/rule-fixtures.data', () => ({
  data: {
    align_equals: {
      canonical : 'basic_run',
      examples  : [{ case: 'nested', titleHtml: 'Nested' }, { case: 'deep', titleHtml: 'Deep' }]
    }
  }
}))

const cardCases = (frontmatter: Record<string, unknown>, rule?: string): string[] =>
  ruleView.pageCompositionCases(frontmatter, rule).map(entry => entry.case)

const exampleCases = (frontmatter: Record<string, unknown>): string[] =>
  ruleView.pageExamples(frontmatter, 'align_equals').map(example => example.case)

describe('casesForRule', () => {
  it('resolves a fixture directory name to its kebab-case slug', () => {
    expect(ruleView.casesForRule('align_equals')).toStrictEqual(['alpha_case', 'beta_case'])
  })

  it('yields nothing for a rule no previewable case activates', () => {
    expect(ruleView.casesForRule('wrap_docstrings')).toStrictEqual([])
  })
})

describe('fixturesForRule', () => {
  it('returns the canonical case and examples registered for a rule', () => {
    expect(ruleView.fixturesForRule('align_equals')).toStrictEqual({
      canonical : 'basic_run',
      examples  : [{ case: 'nested', titleHtml: 'Nested' }, { case: 'deep', titleHtml: 'Deep' }]
    })
  })

  it('names the unregistered rule when the lookup misses', () => {
    expect(() => ruleView.fixturesForRule('align-equals'))
      .toThrow(/Rule "align-equals" not registered/)
  })
})

describe('pageCompositionCases', () => {
  it('lists every previewable case where no rule narrows the set', () => {
    expect(cardCases({})).toStrictEqual(['alpha_case', 'beta_case', 'gamma_case'])
  })

  it('narrows the cases to the ones the named rule takes part in', () => {
    expect(cardCases({}, 'align_equals')).toStrictEqual(['alpha_case', 'beta_case'])
  })

  it('leaves out a case the page embeds inline', () => {
    const page = { inlineFixtures: ['composition/beta_case'] }
    expect(cardCases(page, 'align_equals')).toStrictEqual(['alpha_case'])
    expect(cardCases(page)).toStrictEqual(['alpha_case', 'gamma_case'])
  })

  it('keeps a case whose name an inline embed of another rule shares', () => {
    expect(cardCases({ inlineFixtures: ['align_equals/beta_case'] }, 'align_equals'))
      .toStrictEqual(['alpha_case', 'beta_case'])
  })
})

describe('pageExamples', () => {
  it('lists every example on a page embedding none of them', () => {
    expect(exampleCases({})).toStrictEqual(['nested', 'deep'])
  })

  it('leaves out an example the page embeds inline', () => {
    expect(exampleCases({ inlineFixtures: ['align_equals/nested'] })).toStrictEqual(['deep'])
  })

  it('keeps an example whose name an inline embed of another rule shares', () => {
    expect(exampleCases({ inlineFixtures: ['composition/nested'] }))
      .toStrictEqual(['nested', 'deep'])
  })
})
