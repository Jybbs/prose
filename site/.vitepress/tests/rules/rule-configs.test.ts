import loader        from '../../lib/rules/rule-configs.data'
import { RULE_DEFS } from '../schema'

const configs = await loader.load([])

describe('derived rule configs', () => {
  it('carries a row set for every rule in the schema', () => {
    expect(Object.keys(configs).toSorted()).toStrictEqual(Object.keys(RULE_DEFS).toSorted())
  })

  it.each(Object.keys(RULE_DEFS))('%s mirrors its schema keys and defaults', slug => {
    const rows = configs[slug]
    expect(rows.map(row => row.key).toSorted())
      .toStrictEqual(Object.keys(RULE_DEFS[slug].default).toSorted())
    for (const row of rows) {
      expect(JSON.parse(row.default)).toStrictEqual(RULE_DEFS[slug].default[row.key])
    }
  })

  it.each(Object.keys(RULE_DEFS))('%s leads with enabled and walks every meaning', slug => {
    const rows = configs[slug]
    expect(rows[0].key).toBe('enabled')
    for (const row of rows) {
      expect(row.meaningNodes.length).toBeGreaterThan(0)
      expect(row.typeNodes.length).toBeGreaterThan(0)
    }
  })
})
