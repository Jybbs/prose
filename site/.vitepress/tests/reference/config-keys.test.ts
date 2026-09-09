import loader           from '../../lib/reference/config-keys.data'
import { declaredKeys } from '../../lib/shared/rule-schema'
import { SCHEMA }       from '../schema'

const keys = await loader.load([])

const declared = declaredKeys(SCHEMA)

describe('derived config keys', () => {
  it('covers every top-level schema key outside the nested tables', () => {
    expect(keys.top.map(row => row.key).toSorted()).toStrictEqual(declared.top.toSorted())
  })

  it('mirrors the cache and imports sub-tables', () => {
    expect(keys.cache.map(row => row.key)).toStrictEqual(declared.cache.toSorted())
    expect(keys.imports.map(row => row.key)).toStrictEqual(declared.imports.toSorted())
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
