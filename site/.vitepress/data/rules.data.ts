import { defineLoader } from 'vitepress'

import { getRenderer }         from '../lib/markdown/renderer'
import { discoverRuleSlugs }   from '../lib/rules/discovery'
import type { DiscoveredRule } from '../lib/rules/discovery'
import { rulesDir }            from '../lib/shared/paths'
import { CATEGORY_META, FAMILY_META, FAMILY_ORDER, type RuleCategory, type RuleFamily } from '../lib/shared/registries'
import { toTitleCase }         from '../lib/shared/title-case'

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
  family : RuleFamily
  label  : string
  rules  : readonly RenderedRule[]
}

interface RuleCategoryGroup {
  byFamily : readonly RuleFamilyGroup[]
  category : RuleCategory
  label    : string
}

interface RulesData {
  byCategory : readonly RuleCategoryGroup[]
  byFamily   : Record<RuleFamily, readonly RenderedRule[]>
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
      captionHtml   : md.renderInline(`*Prose* ${r.caption}`),
      categoryBadge : CATEGORY_META[r.category].badge,
      categoryLabel : CATEGORY_META[r.category].label,
      familyBadge   : FAMILY_META[r.family].badge,
      familyLabel   : FAMILY_META[r.family].label,
      name          : toTitleCase(r.slug, '-')
    }))
    const bySlug     = Object.fromEntries(list.map(r => [r.slug, r])) as Record<string, RenderedRule>
    const byFamily   = Object.groupBy(list, r => r.family) as Record<RuleFamily, readonly RenderedRule[]>
    for (const family of FAMILY_ORDER) byFamily[family] ??= []
    const byCategory = (['auto-fix', 'lint'] as const).map(category => {
      const rulesInCategory = list.filter(r => r.category === category)
      const grouped         = Object.groupBy(rulesInCategory, r => r.family) as Partial<Record<RuleFamily, readonly RenderedRule[]>>
      return {
        byFamily : FAMILY_ORDER
          .filter(family => grouped[family]?.length)
          .map(family => ({
            family,
            label : FAMILY_META[family].label,
            rules : grouped[family]!
          })),
        category,
        label    : CATEGORY_META[category].label
      }
    })
    return { byCategory, byFamily, bySlug, list }
  }
})
