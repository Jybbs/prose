import { defineLoader } from 'vitepress'

import { getRenderer } from '../markdown/renderer'
import * as paths      from '../shared/paths'

import { configRow, type ConfigRow, type SchemaProps } from '../shared/rule-schema'

export interface ConfigKeys {
  cache   : readonly ConfigRow[]
  imports : readonly ConfigRow[]
  top     : readonly ConfigRow[]
}

const NESTED = new Set(['cache', 'imports', 'rules'])

const root = paths.repoRoot(import.meta.url)

declare const data: ConfigKeys
export { data }

export default defineLoader({
  watch : paths.proseBinaryCandidates(root),
  async load(): Promise<ConfigKeys> {
    const md     = await getRenderer()
    const schema = JSON.parse(paths.runProse(root, ['schema']))
    const defs   = schema.$defs as Record<string, { properties: SchemaProps }>

    const rows = (props: SchemaProps): readonly ConfigRow[] =>
      Object.entries(props)
        .filter(([key]) => !NESTED.has(key))
        .toSorted(([a], [b]) => a.localeCompare(b))
        .map(([key, prop]) => configRow(md, key, prop, prop.default ?? null))

    return {
      cache   : rows(defs.CacheConfig.properties),
      imports : rows(defs.ImportsConfig.properties),
      top     : rows(schema.properties as SchemaProps)
    }
  }
})
