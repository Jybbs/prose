import { defineLoader } from 'vitepress'

import { getRenderer } from '../markdown/renderer'
import * as paths      from '../shared/paths'

import { configRow, rulePropsOf, type ConfigRow, type SchemaProps } from '../shared/rule-schema'

type RuleConfigData = Record<string, readonly ConfigRow[]>

const root = paths.repoRoot(import.meta.url)

declare const data: RuleConfigData
export { data }

export default defineLoader({
  watch : paths.proseBinaryCandidates(root),
  async load(): Promise<RuleConfigData> {
    const md     = await getRenderer()
    const schema = JSON.parse(paths.runProse(root, ['schema']))
    const defs   = schema.$defs as Record<string, { properties: SchemaProps }>
    const rules  = schema.$defs.RuleConfigs.properties as
      Record<string, { anyOf?: readonly { $ref?: string }[], default: Record<string, unknown> }>

    // `enabled` is documented once, on `ToggleOnly`, whatever sub-table a
    // rule resolves through.
    const enabled = defs.ToggleOnly.properties.enabled

    return Object.fromEntries(Object.entries(rules).map(([slug, def]) => {
      const props = rulePropsOf(defs, def)
      const keys  = Object.keys(def.default).toSorted((a, b) =>
        a === 'enabled' ? -1 : b === 'enabled' ? 1 : a.localeCompare(b))
      return [slug, keys.map(key =>
        configRow(md, key, key === 'enabled' ? enabled : props[key], def.default[key]))]
    }))
  }
})
