import { defineLoader } from 'vitepress'

import { rulesDir }              from '../lib/paths'
import { discoverRuleSlugs }     from '../lib/rules-discovery'
import type { DiscoveredRule }   from '../lib/rules-discovery'
import type { Registry }         from '../lib/types'

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
