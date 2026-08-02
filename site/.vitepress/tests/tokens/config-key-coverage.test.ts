import { repoRoot, runProse } from '../../lib/shared/paths'
import { NESTED_TABLES }      from '../../lib/shared/rule-schema'
import * as sources           from '../../lib/tokens/sources'

const PREFIXES: Record<string, string> = { CacheConfig: 'cache.', ImportsConfig: 'imports.' }

const UNSCHEMED = new Set(['overrides.paths'])

const schema = JSON.parse(runProse(repoRoot(import.meta.url), ['schema']))

const schemaKeys = new Set<string>([
  ...Object.keys(schema.properties).filter(key => !NESTED_TABLES.has(key)),
  ...Object.entries(schema.$defs as Record<string, { properties?: object }>)
    .filter(([name]) => name.endsWith('Config') && name !== 'RuleConfigs')
    .flatMap(([name, def]) =>
      Object.keys(def.properties ?? {}).map(key => (PREFIXES[name] ?? '') + key))
])

const tokenKeys = sources.SOURCES['config-key'].map(source => source.key)

describe('config-key tokens', () => {
  it('names only keys the schema declares', () => {
    expect(tokenKeys.filter(key => !UNSCHEMED.has(key) && !schemaKeys.has(key))).toEqual([])
  })

  it('covers every key the schema declares', () => {
    expect([...schemaKeys].filter(key => !tokenKeys.includes(key)).toSorted()).toEqual([])
  })
})
