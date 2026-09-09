import loader        from '../../lib/reference/facets.data'
import { RULE_DEFS } from '../schema'

const families = await loader.load([])

// The keys the generic group renders are the ones the per-rule lists drop.
const hoisted = new Set(
  (families.find(family => family.family === 'generic')?.rules ?? [])
    .flatMap(group => group.facets.map(facet => facet.key)))

const derived = families
  .filter(family => family.family !== 'generic')
  .flatMap(family => family.rules.flatMap(group =>
    group.facets.map(facet => [group.rule, facet.key, facet.default] as const)))

const expected = Object.entries(RULE_DEFS)
  .flatMap(([rule, def]) => Object.keys(def.default)
    .filter(key => !hoisted.has(key))
    .map(key => `${rule}.${key}`))

describe('derived facets', () => {
  it('covers every schema facet outside the hoisted scopes', () => {
    expect(derived.map(([rule, key]) => `${rule}.${key}`).toSorted())
      .toStrictEqual(expected.toSorted())
  })

  it.each(derived)('%s.%s mirrors the schema default', (rule, key, value) => {
    expect(RULE_DEFS[rule].default[key]).toStrictEqual(JSON.parse(value))
  })

  it.each(derived)('%s.%s carries a walked meaning', (rule, key) => {
    const facet = families
      .flatMap(family => family.rules)
      .find(group => group.rule === rule)
      ?.facets.find(entry => entry.key === key)
    expect(facet?.meaningNodes.length).toBeGreaterThan(0)
  })

  it('anchors every rule group and facet to an id no other one takes', () => {
    const anchors = families.flatMap(family =>
      family.rules.flatMap(group => [group.anchor, ...group.facets.map(facet => facet.anchor)]))
    expect(new Set(anchors).size).toBe(anchors.length)
  })

  it.each(derived)('%s.%s anchors beneath its own rule', (rule, key) => {
    const group = families.flatMap(family => family.rules).find(entry => entry.rule === rule)
    expect(group?.facets.find(facet => facet.key === key)?.anchor)
      .toBe(`${group?.anchor}-${key}`)
  })

  it('hoists the scopes every rule shares', () => {
    const generic = families.find(family => family.family === 'generic')
    expect(generic?.rules.map(group => group.rule)).toStrictEqual(['every rule', 'alignment rules'])
    expect(generic?.rules.flatMap(group => group.facets.map(facet => facet.key)))
      .toStrictEqual(['enabled', 'max-shift'])
    expect(generic?.rules.flatMap(group =>
      group.facets.filter(facet => facet.meaningNodes.length === 0).map(facet => facet.key)))
      .toStrictEqual([])
  })
})
