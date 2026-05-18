import { defineLoader } from 'vitepress'

import { discoverRuleSlugs }   from '../lib/rules/discovery'
import type { DiscoveredRule } from '../lib/rules/discovery'
import { rulesDir }            from '../lib/shared/paths'
import { CATEGORY_META, type RuleCategory } from '../lib/shared/registries'

export type { DiscoveredRule }

export interface RuleCategoryGroup {
  category : RuleCategory
  label    : string
  rules    : readonly DiscoveredRule[]
}

export interface RulesData {
  byCategory : readonly RuleCategoryGroup[]
  bySlug     : Record<string, DiscoveredRule>
  list       : readonly DiscoveredRule[]
}

const rulesDirectory = rulesDir(import.meta.url)

declare const data: RulesData
export { data }

export default defineLoader({
  watch: [`${rulesDirectory}/*.md`],
  async load(): Promise<RulesData> {
    const list       = discoverRuleSlugs(rulesDirectory)
    const bySlug     = Object.fromEntries(list.map(r => [r.slug, r])) as Record<string, DiscoveredRule>
    const byCategory = (['auto-fix', 'lint'] as const).map(category => ({
      category,
      label : CATEGORY_META[category].label,
      rules : list.filter(r => r.category === category)
    }))
    return { byCategory, bySlug, list }
  }
})
