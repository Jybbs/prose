import { groupRules }                   from '../lib/rules/grouping'
import type { RenderedRule, RulesData } from '../lib/rules/grouping'
import * as registries                  from '../lib/shared/registries'
import { ruleRoute }                    from '../lib/shared/routes'
import { toTitleCase }                  from '../lib/shared/title-case'

interface RuleStub extends Partial<RenderedRule> {
  slug : string
}

// Builds the `rules.data` module shape from a slug and whatever a case needs
// to differ, deriving each rule's route, badges, and labels before handing the
// list to `groupRules`.
export function rulesDataStub(stubs: readonly RuleStub[] = []): { data: RulesData } {
  return { data: groupRules(stubs.map(renderedRule)) }
}

function renderedRule(stub: RuleStub): RenderedRule {
  const caption  = `What ${stub.slug} does.`
  const family   = stub.family ?? 'alignment'
  const category = stub.category ?? registries.categoryOf(family)
  return {
    caption       : caption,
    captionHtml   : caption,
    category      : category,
    categoryBadge : registries.CATEGORY_META[category].badge,
    categoryLabel : registries.CATEGORY_META[category].label,
    family        : family,
    familyBadge   : registries.FAMILY_META[family].badge,
    familyLabel   : registries.FAMILY_META[family].label,
    href          : ruleRoute(family, stub.slug),
    lints         : category === 'lint',
    name          : toTitleCase(stub.slug, '-'),
    related       : [],
    ...stub
  }
}
