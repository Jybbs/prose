import { repoRoot, runProse } from '../../lib/shared/paths'
import { rulePropsOf }        from '../../lib/shared/rule-schema'
import * as sources           from '../../lib/tokens/sources'

const token = (key: string, domain: sources.Domain): sources.Token =>
  ({ blurbNodes: [], domain, href: '', key, sort: key })

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

describe('config-key sources', () => {
  it('indexes every per-rule facet the schema declares', () => {
    const schema = JSON.parse(runProse(repoRoot(import.meta.url), ['schema']))
    const defs   = schema.$defs as Record<string, { properties: Record<string, never> }>
    const rules  = schema.$defs.RuleConfigs.properties as
      Record<string, { anyOf?: readonly { $ref?: string }[] }>

    const declared = Object.values(rules).flatMap(def => Object.keys(rulePropsOf(defs, def)))
    const indexed  = new Set(sources.SOURCES['config-key'].map(source => source.key))

    expect(declared.filter(key => !indexed.has(key)).toSorted()).toEqual([])
  })
})
