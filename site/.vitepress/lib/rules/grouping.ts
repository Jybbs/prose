import { groupByMember }        from '../shared/group-by-member'
import * as registries          from '../shared/registries'
import type { DiscoveredRule }  from './discovery'

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

export interface RulesData {
  byCategory : readonly RuleCategoryGroup[]
  byFamily   : Record<registries.RuleFamily, readonly RenderedRule[]>
  bySlug     : Record<string, RenderedRule>
  list       : readonly RenderedRule[]
}

// Indexes a rendered rule list by slug, by family, and by category, giving
// every family an entry even where no rule carries it.
export function groupRules(list: readonly RenderedRule[]): RulesData {
  const byFamily   = groupByMember(list, rule => rule.family, registries.FAMILY_ORDER)
  const bySlug     = Object.fromEntries(list.map(rule => [rule.slug, rule]))
  const byCategory = registries.CATEGORY_ORDER
    .map(category => ({
      byFamily : registries.FAMILY_ORDER
        .map(family => ({
          family,
          label : registries.FAMILY_META[family].label,
          rules : byFamily[family].filter(rule => rule.category === category)
        }))
        .filter(group => group.rules.length > 0),
      category,
      label    : registries.CATEGORY_META[category].label
    }))
  return { byCategory, byFamily, bySlug, list }
}
