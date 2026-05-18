import { defineLoader } from 'vitepress'

import { discoverRuleSlugs }   from '../lib/rules/discovery'
import type { DiscoveredRule } from '../lib/rules/discovery'
import { rulesDir }            from '../lib/shared/paths'
import type { Registry }       from '../lib/shared/types'

export type { DiscoveredRule }

export interface RulesData {
  bySlug : Registry<DiscoveredRule>
  list   : readonly DiscoveredRule[]
}

const rulesDirectory = rulesDir(import.meta.url)

declare const data: RulesData
export { data }

export default defineLoader({
  watch: [`${rulesDirectory}/*.md`],
  async load(): Promise<RulesData> {
    const list   = discoverRuleSlugs(rulesDirectory)
    const bySlug = Object.fromEntries(list.map(r => [r.slug, r])) as Registry<DiscoveredRule>
    return { bySlug, list }
  }
})
