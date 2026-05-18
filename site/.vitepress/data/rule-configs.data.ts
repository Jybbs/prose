import { defineLoader } from 'vitepress'

import { getRenderer, renderInlineField } from '../lib/markdown/renderer'
import { RULE_CONFIG_PRESETS, type RuleConfigPreset } from '../lib/rules/config-presets'

export interface RuleConfigRow {
  default     : string
  key         : string
  meaningHtml : string
  typeHtml    : string
}

export type RuleConfigData = Record<RuleConfigPreset, readonly RuleConfigRow[]>

declare const data: RuleConfigData
export { data }

export default defineLoader({
  watch: [],
  async load(): Promise<RuleConfigData> {
    const md   = await getRenderer()
    const out  = {} as Record<RuleConfigPreset, readonly RuleConfigRow[]>
    for (const [preset, rows] of Object.entries(RULE_CONFIG_PRESETS) as Array<[RuleConfigPreset, typeof RULE_CONFIG_PRESETS[RuleConfigPreset]]>) {
      const withType    = renderInlineField(md, rows, 'type')
      const withBoth    = renderInlineField(md, withType, 'meaning')
      out[preset]       = withBoth
    }
    return out
  }
})
