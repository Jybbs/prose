import { defineLoader } from 'vitepress'

import { discoverRuleSlugs }   from '../lib/rules/discovery'
import type { DiscoveredRule } from '../lib/rules/discovery'
import { rulesDir }            from '../lib/shared/paths'
import { CATEGORY_META, DOMAIN_META, type RuleCategory, type RuleDomain } from '../lib/shared/registries'

export type { DiscoveredRule }

export interface RuleDomainGroup {
  domain : RuleDomain
  label  : string
  rules  : readonly DiscoveredRule[]
}

export interface RuleCategoryGroup {
  byDomain : readonly RuleDomainGroup[]
  category : RuleCategory
  label    : string
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
    const byCategory = (['auto-fix', 'lint'] as const).map(category => {
      const rulesInCategory = list.filter(r => r.category === category)
      const byDomain = (Object.keys(DOMAIN_META) as RuleDomain[])
        .filter(domain => rulesInCategory.some(r => r.domain === domain))
        .map(domain => ({
          domain,
          label : DOMAIN_META[domain].label,
          rules : rulesInCategory.filter(r => r.domain === domain)
        }))
      return {
        byDomain,
        category,
        label: CATEGORY_META[category].label
      }
    })
    return { byCategory, bySlug, list }
  }
})
