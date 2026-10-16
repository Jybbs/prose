import { groupByMember }                 from '../shared/group-by-member'
import { FAMILY_ORDER, type RuleFamily } from '../shared/registries'
import type { DiscoveredRule }           from './discovery'

export interface RenderedRule extends DiscoveredRule {
  captionHtml : string
  familyLabel : string
  name        : string
}

export interface RulesData {
  byFamily : Record<RuleFamily, readonly RenderedRule[]>
  bySlug   : Record<string, RenderedRule>
  list     : readonly RenderedRule[]
}

// Indexes a rendered rule list by slug and by family, giving every family an
// entry even where no rule carries it.
export function groupRules(list: readonly RenderedRule[]): RulesData {
  const byFamily = groupByMember(list, rule => rule.family, FAMILY_ORDER)
  const bySlug   = Object.fromEntries(list.map(rule => [rule.slug, rule]))
  return { byFamily, bySlug, list }
}
