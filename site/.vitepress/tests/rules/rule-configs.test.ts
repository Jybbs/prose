import loader                 from '../../lib/rules/rule-configs.data'
import { repoRoot, runProse } from '../../lib/shared/paths'

const configs = await loader.load([])

const ruleDefs = JSON.parse(runProse(repoRoot(import.meta.url), ['schema']))
  .$defs.RuleConfigs.properties as Record<string, { default: Record<string, unknown> }>

describe('derived rule configs', () => {
  it('carries a row set for every rule in the schema', () => {
    expect(Object.keys(configs).toSorted()).toEqual(Object.keys(ruleDefs).toSorted())
  })

  it.each(Object.keys(ruleDefs))('%s mirrors its schema keys and defaults', slug => {
    const rows = configs[slug]
    expect(rows.map(row => row.key).toSorted())
      .toEqual(Object.keys(ruleDefs[slug].default).toSorted())
    for (const row of rows) {
      expect(JSON.parse(row.default)).toEqual(ruleDefs[slug].default[row.key])
    }
  })

  it.each(Object.keys(ruleDefs))('%s leads with enabled and walks every meaning', slug => {
    const rows = configs[slug]
    expect(rows[0].key).toBe('enabled')
    for (const row of rows) {
      expect(row.meaningNodes.length).toBeGreaterThan(0)
      expect(row.typeNodes.length).toBeGreaterThan(0)
    }
  })
})
