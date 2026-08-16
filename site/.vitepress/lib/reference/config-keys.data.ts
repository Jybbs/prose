import { defineLoader } from 'vitepress'

import { getRenderer } from '../markdown/renderer'
import * as paths      from '../shared/paths'
import * as ruleSchema from '../shared/rule-schema'

export interface ConfigKeys {
  cache   : readonly ruleSchema.ConfigRow[]
  imports : readonly ruleSchema.ConfigRow[]
  top     : readonly ruleSchema.ConfigRow[]
}

const root = paths.repoRoot(import.meta.url)

declare const data: ConfigKeys
export { data }

export default defineLoader({
  watch : [paths.proseBinaryPath(root)],
  async load(): Promise<ConfigKeys> {
    const md     = await getRenderer()
    const schema = ruleSchema.proseSchema(root)
    const defs   = schema.$defs

    const rows = (props: ruleSchema.SchemaProps): readonly ruleSchema.ConfigRow[] =>
      Object.entries(props)
        .filter(([key]) => !ruleSchema.NESTED_TABLES.has(key))
        .toSorted(([a], [b]) => a.localeCompare(b))
        .map(([key, prop]) => ruleSchema.configRow(md, key, prop, prop.default ?? null))

    return {
      cache   : rows(defs.CacheConfig.properties),
      imports : rows(defs.ImportsConfig.properties),
      top     : rows(schema.properties)
    }
  }
})
