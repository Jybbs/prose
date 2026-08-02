import { repoRoot, runProse } from '../../lib/shared/paths'
import * as sources           from '../../lib/tokens/sources'

const NESTED = new Set(['cache', 'imports', 'rules'])

// `[[tool.prose.overrides]]` carries no schema entry, so its key is the
// one row the schema cannot account for.
const UNSCHEMED = ['overrides.paths']

const schema = JSON.parse(runProse(repoRoot(import.meta.url), ['schema']))

const token = (key: string, domain: sources.Domain): sources.Token =>
  ({ blurbNodes: [], domain, href: '', key, sort: key })

function schemaConfigKeys(): string[] {
  const defs   = schema.$defs
  const rules  = defs.RuleConfigs.properties as Record<string, { default: Record<string, unknown> }>
  const facets = new Set(Object.values(rules).flatMap(rule => Object.keys(rule.default)))
  return [
    ...Object.keys(schema.properties).filter(key => !NESTED.has(key)),
    ...Object.keys(defs.CacheConfig.properties).map(key => `cache.${key}`),
    ...Object.keys(defs.ImportsConfig.properties).map(key => `imports.${key}`),
    ...facets,
    ...UNSCHEMED
  ]
}

describe('config-key sources', () => {
  it('lists every key the schema declares and nothing beyond it', () => {
    expect(sources.SOURCES['config-key'].map(s => s.key).toSorted())
      .toEqual(schemaConfigKeys().toSorted())
  })
})

describe('stripPrefix', () => {
  it.each([
    ['# fmt: off',                   'off'],
    ['# prose: ignore[<rule>, ...]', 'ignore[<rule>, ...]'],
    ['# yapf: disable',              'disable'],
    ['--color',                      'color'],
    ['prose check',                  'check']
  ])('reduces %s to its sort key', (input, expected) => {
    expect(sources.stripPrefix(input)).toBe(expected)
  })
})

describe('groupByDomain', () => {
  it('buckets by domain, both the buckets and their tokens sorted', () => {
    const tokens = [
      token('z', 'config-key'), token('a', 'cli-flag'), token('y', 'config-key'), token('b', 'cli-flag')
    ]
    expect(sources.groupByDomain(tokens)).toEqual([
      ['cli-flag',   [token('a', 'cli-flag'), token('b', 'cli-flag')]],
      ['config-key', [token('y', 'config-key'), token('z', 'config-key')]]
    ])
  })

  it('does not mutate its input', () => {
    const input = [token('b', 'cli-flag'), token('a', 'cli-flag')]
    sources.groupByDomain(input)
    expect(input.map(t => t.key)).toEqual(['b', 'a'])
  })
})
