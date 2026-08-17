import { defineLoader } from 'vitepress'

import { getRenderer } from '../markdown/renderer'
import * as paths      from '../shared/paths'
import * as ruleSchema from '../shared/rule-schema'

export type ConfigKeys = Record<ruleSchema.ConfigSection, readonly ruleSchema.ConfigRow[]>

const root = paths.repoRoot(import.meta.url)

declare const data: ConfigKeys
export { data }

export default defineLoader({
  watch : [paths.proseBinaryPath(root)],
  async load(): Promise<ConfigKeys> {
    const md       = await getRenderer()
    const sections = ruleSchema.sectionProps(ruleSchema.proseSchema(root))

    const rows = (props: ruleSchema.SchemaProps): readonly ruleSchema.ConfigRow[] =>
      Object.entries(props)
        .toSorted(([a], [b]) => a.localeCompare(b))
        .map(([key, prop]) => ruleSchema.configRow(md, key, prop, prop.default ?? null))

    return {
      cache   : rows(sections.cache),
      imports : rows(sections.imports),
      top     : rows(sections.top)
    }
  }
})
