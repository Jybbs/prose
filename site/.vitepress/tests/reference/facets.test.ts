import { SOURCES }            from '../../lib/reference/facets.data'
import { repoRoot, runProse } from '../../lib/shared/paths'

const ruleDefs = JSON.parse(runProse(repoRoot(import.meta.url), ['schema']))
  .$defs.RuleConfigs.properties as Record<string, { default: Record<string, unknown> }>

const curated = SOURCES
  .filter(family => family.family !== 'generic')
  .flatMap(family => family.rules.flatMap(group =>
    group.facets.map(facet => [group.rule, facet.key, facet.default] as const)))

describe('facet sources', () => {
  it.each(curated)('%s.%s mirrors the schema default', (rule, key, value) => {
    expect(ruleDefs[rule].default[key]).toEqual(JSON.parse(value))
  })
})
