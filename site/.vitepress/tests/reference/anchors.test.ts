import { facetAnchor, hoistedFacetAnchor, ruleAnchor } from '../../lib/reference/anchors'

describe('ruleAnchor', () => {
  it.each([
    ['alignment rules',    'alignment-rules'],
    ['every rule',         'every-rule'],
    ['reflow-collections', 'reflow-collections']
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

describe('hoistedFacetAnchor', () => {
  it.each([
    ['enabled',   'every-rule-enabled'],
    ['max-shift', 'alignment-rules-max-shift']
  ])('sends %s to its scope anchor', (key, expected) => {
    expect(hoistedFacetAnchor(key)).toBe(expected)
  })

  it('declines a facet the catalogue lists under a rule of its own', () => {
    expect(hoistedFacetAnchor('max-args')).toBeUndefined()
  })
})
