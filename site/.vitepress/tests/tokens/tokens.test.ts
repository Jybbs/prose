import * as sources           from '../../lib/tokens/sources'
import { repoRoot, runProse } from '../../lib/shared/paths'

const token = (key: string, domain: sources.Domain): sources.Token =>
  ({ blurbNodes: [], domain, href: '', key, sort: key })

const schema   = JSON.parse(runProse(repoRoot(import.meta.url), ['schema']))
const NESTED   = new Set(['cache', 'imports', 'rules'])
const ruleDefs = schema.$defs.RuleConfigs.properties as
  Record<string, { default: Record<string, unknown> }>

const schemaKeys = new Set([
  ...Object.keys(schema.properties).filter(key => !NESTED.has(key)),
  ...Object.values(ruleDefs).flatMap(def => Object.keys(def.default))
])

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

describe('config-key tokens', () => {
  it('carries every key the schema declares', () => {
    const listed = new Set(sources.SOURCES['config-key'].map(entry => entry.key))
    expect([...schemaKeys].filter(key => !listed.has(key)).toSorted()).toEqual([])
  })
})
