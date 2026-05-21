import { defineLoader } from 'vitepress'

import { getRenderer }         from '../lib/markdown/renderer'
import { discoverRuleSlugs }   from '../lib/rules/discovery'
import type { DiscoveredRule } from '../lib/rules/discovery'
import { rulesDir }            from '../lib/shared/paths'
import { CATEGORY_META, FAMILY_META, type RuleCategory, type RuleFamily } from '../lib/shared/registries'
import { toTitleCase }         from '../lib/shared/title-case'

export type { DiscoveredRule }

export interface RenderedRule extends DiscoveredRule {
  captionHtml : string
  name        : string
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
      captionHtml: md.renderInline(`*Prose* ${r.caption}`),
      name       : toTitleCase(r.slug, '-')
    }))
    const bySlug     = Object.fromEntries(list.map(r => [r.slug, r])) as Record<string, RenderedRule>
    const families   = Object.keys(FAMILY_META) as RuleFamily[]
    const byFamily   = Object.groupBy(list, r => r.family) as Record<RuleFamily, readonly RenderedRule[]>
    for (const family of families) byFamily[family] ??= []
    const byCategory = (['auto-fix', 'lint'] as const).map(category => {
      const rulesInCategory = list.filter(r => r.category === category)
      const grouped         = Object.groupBy(rulesInCategory, r => r.family) as Partial<Record<RuleFamily, readonly RenderedRule[]>>
      return {
        byFamily : families
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
