import { defineLoader } from 'vitepress'

import { getRenderer }               from '../markdown/renderer'
import { type ConfigRow, configRow } from '../shared/config-row'
import * as paths                    from '../shared/paths'
import * as ruleSchema               from '../shared/rule-schema'

type RuleConfigData = Record<string, readonly ConfigRow[]>

const root = paths.repoRoot(import.meta.url)

declare const data: RuleConfigData
export { data }

export default defineLoader({
  watch : [paths.proseBinaryPath(root)],
  async load(): Promise<RuleConfigData> {
    const md     = await getRenderer()
    const schema = ruleSchema.proseSchema(root)
    const defs   = schema.$defs
    const rules  = ruleSchema.ruleDefsOf(schema)

    return Object.fromEntries(Object.entries(rules).map(([slug, def]) => {
      const props = ruleSchema.facetPropsOf(defs, def)
      const keys  = ruleSchema.facetKeys(def.default)
      return [slug, keys.map(key => configRow(md, key, props[key], def.default[key]))]
    }))
  }
})
