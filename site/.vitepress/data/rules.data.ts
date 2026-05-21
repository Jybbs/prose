import { defineLoader } from 'vitepress'

import { getRenderer }         from '../lib/markdown/renderer'
import { discoverRuleSlugs }   from '../lib/rules/discovery'
import type { DiscoveredRule } from '../lib/rules/discovery'
import { rulesDir }            from '../lib/shared/paths'
import { CATEGORY_META, FAMILY_META, type RuleCategory, type RuleFamily } from '../lib/shared/registries'

export type { DiscoveredRule }

export interface RenderedRule extends DiscoveredRule {
  captionHtml : string
}

export interface RuleFamilyGroup {
  family : RuleFamily
  label  : string
  rules  : readonly RenderedRule[]
}

export interface RuleCategoryGroup {
  byFamily : readonly RuleFamilyGroup[]
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
      const byFamily = (Object.keys(FAMILY_META) as RuleFamily[])
        .filter(family => rulesInCategory.some(r => r.family === family))
        .map(family => ({
          family,
          label : FAMILY_META[family].label,
          rules : rulesInCategory.filter(r => r.family === family)
        }))
      return {
        byFamily,
        category,
        label: CATEGORY_META[category].label
      }
    })
    return { byCategory, bySlug, list }
  }
})
