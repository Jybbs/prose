import { defineLoader } from 'vitepress'

import { getRenderer, renderInlineHtml } from '../markdown/renderer'
import { discoverRuleSlugs }             from './discovery'
import type { DiscoveredRule }           from './discovery'
import { rulesDir }                      from '../shared/paths'
import * as registries                   from '../shared/registries'
import { toTitleCase }                   from '../shared/title-case'

export type { DiscoveredRule }

export interface RenderedRule extends DiscoveredRule {
  captionHtml   : string
  categoryBadge : string
  categoryLabel : string
  familyBadge   : string
  familyLabel   : string
  name          : string
}

interface RuleFamilyGroup {
  family : registries.RuleFamily
  label  : string
  rules  : readonly RenderedRule[]
}

interface RuleCategoryGroup {
  byFamily : readonly RuleFamilyGroup[]
  category : registries.RuleCategory
  label    : string
}

interface RulesData {
  byCategory : readonly RuleCategoryGroup[]
  byFamily   : Record<registries.RuleFamily, readonly RenderedRule[]>
  bySlug     : Record<string, RenderedRule>
  list       : readonly RenderedRule[]
}

const rulesDirectory = rulesDir(import.meta.url)

declare const data: RulesData
export { data }

export default defineLoader({
  watch: [`${rulesDirectory}/*/*.md`],
  async load(): Promise<RulesData> {
    const md         = await getRenderer()
    const list       = discoverRuleSlugs(rulesDirectory).map(r => ({
      ...r,
      captionHtml   : renderInlineHtml(md, r.caption),
      categoryBadge : registries.CATEGORY_META[r.category].badge,
      categoryLabel : registries.CATEGORY_META[r.category].label,
      familyBadge   : registries.FAMILY_META[r.family].badge,
      familyLabel   : registries.FAMILY_META[r.family].label,
      name          : toTitleCase(r.slug, '-')
    }))
    const bySlug     = Object.fromEntries(list.map(r => [r.slug, r])) as Record<string, RenderedRule>
    type ByFamily    = Record<registries.RuleFamily, readonly RenderedRule[]>
    const byFamily   = Object.groupBy(list, r => r.family) as ByFamily
    for (const family of registries.FAMILY_ORDER) byFamily[family] ??= []
    const categories = Object.keys(registries.CATEGORY_META) as registries.RuleCategory[]
    const byCategory = categories.map(category => {
      const rulesInCategory = list.filter(r => r.category === category)
      type GroupedRules     = Partial<Record<registries.RuleFamily, readonly RenderedRule[]>>
      const grouped         = Object.groupBy(rulesInCategory, r => r.family) as GroupedRules
      return {
        byFamily : registries.FAMILY_ORDER
          .filter(family => grouped[family]?.length)
          .map(family => ({
            family,
            label : registries.FAMILY_META[family].label,
            rules : grouped[family]!
          })),
        category,
        label    : registries.CATEGORY_META[category].label
      }
    })
    return { byCategory, byFamily, bySlug, list }
  }
})
