import loader        from '../../lib/rules/rule-configs.data'
import { RULE_DEFS } from '../schema'

const configs = await loader.load([])

describe('derived rule configs', () => {
  it('carries a row set for every rule in the schema', () => {
    expect(Object.keys(configs).toSorted()).toStrictEqual(Object.keys(RULE_DEFS).toSorted())
  })

  it.each(Object.keys(RULE_DEFS))('%s mirrors its schema keys and defaults', slug => {
    expect(Object.fromEntries(configs[slug].map(row => [row.key, JSON.parse(row.default) as unknown])))
      .toStrictEqual(RULE_DEFS[slug].default)
  })

  it.each(Object.keys(RULE_DEFS))('%s leads with enabled', slug => {
    expect(configs[slug][0].key).toBe('enabled')
  })

  it.each(Object.entries(configs).flatMap(([slug, rows]) => rows.map(row => [slug, row.key, row] as const)))(
    '%s %s renders its meaning and its type',
    (_slug, _key, row) => {
      expect(row.meaningNodes).not.toHaveLength(0)
      expect(row.typeNodes).not.toHaveLength(0)
    }
  )
})
