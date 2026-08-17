import { repoRoot }                  from '../../lib/shared/paths'
import { declaredKeys, proseSchema } from '../../lib/shared/rule-schema'
import * as sources                  from '../../lib/tokens/sources'

const token = (domain: sources.Domain, key: string): sources.Token =>
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
      token('config-key', 'z'), token('cli-flag', 'a'), token('config-key', 'y'), token('cli-flag', 'b')
    ]
    expect(sources.groupByDomain(tokens)).toEqual([
      ['cli-flag',   [token('cli-flag', 'a'), token('cli-flag', 'b')]],
      ['config-key', [token('config-key', 'y'), token('config-key', 'z')]]
    ])
  })

  it('does not mutate its input', () => {
    const input = [token('cli-flag', 'b'), token('cli-flag', 'a')]
    sources.groupByDomain(input)
    expect(input.map(t => t.key)).toEqual(['b', 'a'])
  })
})

describe('config-key sources', () => {
  const keys     = declaredKeys(proseSchema(repoRoot(import.meta.url)))
  const declared = new Set([
    ...keys.top,
    ...keys.rules,
    ...keys.cache.map(key => `cache.${key}`),
    ...keys.imports.map(key => `imports.${key}`)
  ])
  const indexed = sources.SOURCES['config-key'].map(source => source.key)

  it('indexes every key the schema declares', () => {
    expect([...declared].filter(key => !indexed.includes(key)).toSorted()).toEqual([])
  })

  it('indexes nothing the schema leaves out, beyond the overrides table', () => {
    const unschemed = new Set(['overrides.paths'])
    expect(indexed.filter(key => !declared.has(key) && !unschemed.has(key))).toEqual([])
  })
})
