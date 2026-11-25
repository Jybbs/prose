import { groupRules }                   from '../../site/.vitepress/lib/rules/grouping'
import type { RenderedRule, RulesData } from '../../site/.vitepress/lib/rules/grouping'
import * as registries                  from '../../site/.vitepress/lib/shared/registries'
import { ruleRoute }                    from '../../site/.vitepress/lib/shared/routes'
import { toTitleCase }                  from '../../site/.vitepress/lib/shared/title-case'

interface RuleStub extends Partial<RenderedRule> {
  slug : string
}

// Builds the `rules.data` module shape from a slug and whatever a case needs
// to differ, deriving each rule's route and label before handing the list to
// `groupRules`.
export function rulesDataStub(stubs: readonly RuleStub[] = []): { data: RulesData } {
  return { data: groupRules(stubs.map(renderedRule)) }
}

function renderedRule(stub: RuleStub): RenderedRule {
  const caption = `What ${stub.slug} does.`
  const family  = stub.family ?? 'alignment'
  return {
    caption     : caption,
    captionHtml : caption,
    family      : family,
    familyLabel : registries.FAMILY_META[family].label,
    href        : ruleRoute(family, stub.slug),
    lints       : family === 'lint',
    name        : toTitleCase(stub.slug, '-'),
    related     : [],
    ...stub
  }
}
