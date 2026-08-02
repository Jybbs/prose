import loader                 from '../../lib/reference/config-keys.data'
import { repoRoot, runProse } from '../../lib/shared/paths'
import { NESTED_TABLES }      from '../../lib/shared/rule-schema'

const keys = await loader.load([])

const schema = JSON.parse(runProse(repoRoot(import.meta.url), ['schema']))

const topKeys = Object.keys(schema.properties).filter(key => !NESTED_TABLES.has(key))

describe('derived config keys', () => {
  it('covers every top-level schema key outside the nested tables', () => {
    expect(keys.top.map(row => row.key).toSorted()).toEqual(topKeys.toSorted())
  })

  it('mirrors the cache and imports sub-tables', () => {
    expect(keys.cache.map(row => row.key)).toEqual(['enabled', 'max-size-mib'])
    expect(keys.imports.map(row => row.key)).toEqual(['first-party'])
  })

  it('renders a null default as unset', () => {
    const target = keys.top.find(row => row.key === 'target-version')
    expect(target?.default).toBe('unset')
  })

  it.each(['top', 'cache', 'imports'] as const)('%s rows all walk their prose', section => {
    for (const row of keys[section]) {
      expect(row.meaningNodes.length).toBeGreaterThan(0)
      expect(row.typeNodes.length).toBeGreaterThan(0)
    }
  })
})
