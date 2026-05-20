import { defineLoader } from 'vitepress'

import { discoverRuleSlugs }   from '../lib/rules/discovery'
import type { DiscoveredRule } from '../lib/rules/discovery'
import { getRenderer }         from '../lib/markdown/renderer'
import { rulesDir }            from '../lib/shared/paths'
import { CATEGORY_META, DOMAIN_META, type RuleCategory, type RuleDomain } from '../lib/shared/registries'

export type { DiscoveredRule }

export interface RenderedRule extends DiscoveredRule {
  captionHtml : string
}

export interface RuleDomainGroup {
  domain : RuleDomain
  label  : string
  rules  : readonly RenderedRule[]
}

export interface RuleCategoryGroup {
  byDomain : readonly RuleDomainGroup[]
  category : RuleCategory
  label    : string
}

export interface RulesData {
  byCategory : readonly RuleCategoryGroup[]
  bySlug     : Record<string, RenderedRule>
  list       : readonly RenderedRule[]
}

const rulesDirectory = rulesDir(import.meta.url)

declare const data: RulesData
export { data }

export default defineLoader({
  watch: [`${rulesDirectory}/*.md`],
  async load(): Promise<RulesData> {
    const md         = await getRenderer()
    const list       = discoverRuleSlugs(rulesDirectory).map(r => ({
      ...r,
      captionHtml: md.renderInline(r.caption)
    }))
    const bySlug     = Object.fromEntries(list.map(r => [r.slug, r])) as Record<string, RenderedRule>
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
