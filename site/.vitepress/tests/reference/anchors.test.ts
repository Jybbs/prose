import { facetAnchor, ruleAnchor } from '../../lib/reference/anchors'

describe('ruleAnchor', () => {
  it.each([
    ['reflow-collections', 'reflow-collections'],
    ['every rule',         'every-rule'],
    ['alignment rules',    'alignment-rules']
  ])('slugs the %s group to %s', (rule, expected) => {
    expect(ruleAnchor(rule)).toBe(expected)
  })
})

describe('facetAnchor', () => {
  it('sits each facet under its own rule, so two rules sharing a key stay apart', () => {
    expect(facetAnchor('bare-imports', 'allow')).toBe('bare-imports-allow')
    expect(facetAnchor('reassigned-constants', 'allow')).toBe('reassigned-constants-allow')
  })

  it('slugs a hoisted scope on both sides of the join', () => {
    expect(facetAnchor('every rule', 'enabled')).toBe('every-rule-enabled')
    expect(facetAnchor('alignment rules', 'max-shift')).toBe('alignment-rules-max-shift')
  })
})
