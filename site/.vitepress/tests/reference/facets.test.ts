import loader                        from '../../lib/reference/facets.data'
import { repoRoot }                  from '../../lib/shared/paths'
import { proseSchema, type RuleDef } from '../../lib/shared/rule-schema'

const families = await loader.load([])

const ruleDefs = proseSchema(repoRoot(import.meta.url))
  .$defs.RuleConfigs.properties as Record<string, RuleDef>

const derived = families
  .filter(family => family.family !== 'generic')
  .flatMap(family => family.rules.flatMap(group =>
    group.facets.map(facet => [group.rule, facet.key, facet.default] as const)))

const expected = Object.entries(ruleDefs)
  .flatMap(([rule, def]) => Object.keys(def.default)
    .filter(key => key !== 'enabled' && key !== 'max-shift')
    .map(key => `${rule}.${key}`))

describe('derived facets', () => {
  it('covers every schema facet outside the hoisted scopes', () => {
    expect(derived.map(([rule, key]) => `${rule}.${key}`).toSorted())
      .toEqual(expected.toSorted())
  })

  it.each(derived)('%s.%s mirrors the schema default', (rule, key, value) => {
    expect(ruleDefs[rule].default[key]).toEqual(JSON.parse(value))
  })

  it.each(derived)('%s.%s carries a walked meaning', (rule, key) => {
    const facet = families
      .flatMap(family => family.rules)
      .find(group => group.rule === rule)
      ?.facets.find(entry => entry.key === key)
    expect(facet?.meaningNodes.length).toBeGreaterThan(0)
  })

  it('hoists the scopes every rule shares', () => {
    const generic = families.find(family => family.family === 'generic')
    expect(generic?.rules.map(group => group.rule)).toEqual(['every rule', 'alignment rules'])
    expect(generic?.rules.flatMap(group => group.facets.map(facet => facet.key)))
      .toEqual(['enabled', 'max-shift'])
  })
})
