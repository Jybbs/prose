import loader                 from '../../lib/reference/config-keys.data'
import { repoRoot, runProse } from '../../lib/shared/paths'
import { declaredKeys }       from '../../lib/shared/rule-schema'

const keys = await loader.load([])

const declared = declaredKeys(JSON.parse(runProse(repoRoot(import.meta.url), ['schema'])))

describe('derived config keys', () => {
  it('covers every top-level schema key outside the nested tables', () => {
    expect(keys.top.map(row => row.key).toSorted()).toEqual(declared.top.toSorted())
  })

  it('mirrors the cache and imports sub-tables', () => {
    expect(keys.cache.map(row => row.key)).toEqual(declared.cache.toSorted())
    expect(keys.imports.map(row => row.key)).toEqual(declared.imports.toSorted())
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
