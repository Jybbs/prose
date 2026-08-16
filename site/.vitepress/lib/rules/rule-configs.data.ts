import { defineLoader } from 'vitepress'

import { getRenderer } from '../markdown/renderer'
import * as paths      from '../shared/paths'
import * as ruleSchema from '../shared/rule-schema'

type RuleConfigData = Record<string, readonly ruleSchema.ConfigRow[]>

const root = paths.repoRoot(import.meta.url)

declare const data: RuleConfigData
export { data }

export default defineLoader({
  watch : [paths.proseBinaryPath(root)],
  async load(): Promise<RuleConfigData> {
    const md     = await getRenderer()
    const schema = ruleSchema.proseSchema(root)
    const defs   = schema.$defs
    const rules  = defs.RuleConfigs.properties as Record<string, ruleSchema.RuleDef>

    // `enabled` is documented once, on `ToggleOnly`, whatever sub-table a
    // rule resolves through.
    const enabled = defs.ToggleOnly.properties.enabled

    return Object.fromEntries(Object.entries(rules).map(([slug, def]) => {
      const props = ruleSchema.rulePropsOf(defs, def)
      const keys  = ruleSchema.facetKeys(def.default)
      return [slug, keys.map(key =>
        ruleSchema.configRow(md, key, key === 'enabled' ? enabled : props[key], def.default[key]))]
    }))
  }
})
