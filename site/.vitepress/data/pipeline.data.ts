import { defineLoader } from 'vitepress'

import { discoverRuleSlugs }             from '../lib/rules/discovery'
import { parsePipeline, ruleSourcePath } from '../lib/rules/pipeline-source'
import { rulesDir }                      from '../lib/shared/paths'
import { CATEGORY_META, FAMILY_META }    from '../lib/shared/registries'
import type { RuleCategory, RuleFamily } from '../lib/shared/registries'

interface PipelineRule {
  category      : RuleCategory | null
  categoryBadge : string | null
  categoryLabel : string | null
  documented    : boolean
  family        : RuleFamily | null
  familyBadge   : string | null
  familyLabel   : string | null
  imperative    : string
  position      : number
  slug          : string
}

interface PipelineData {
  rules : readonly PipelineRule[]
}

const ruleSource     = ruleSourcePath(import.meta.url)
const rulesDirectory = rulesDir(import.meta.url)

declare const data: PipelineData
export { data }

export default defineLoader({
  watch: [ruleSource, `${rulesDirectory}/*.md`],
  async load(): Promise<PipelineData> {
    const discovered = new Map(discoverRuleSlugs(rulesDirectory).map(r => [r.slug, r]))
    const rules      = parsePipeline(import.meta.url).map(({ imperative, position, slug }) => {
      const entry = discovered.get(slug)
      return {
        category      : entry?.category ?? null,
        categoryBadge : entry ? CATEGORY_META[entry.category].badge : null,
        categoryLabel : entry ? CATEGORY_META[entry.category].label : null,
        documented    : entry !== undefined,
        family        : entry?.family ?? null,
        familyBadge   : entry ? FAMILY_META[entry.family].badge : null,
        familyLabel   : entry ? FAMILY_META[entry.family].label : null,
        imperative,
        position,
        slug
      }
    })
    return { rules }
  }
})
