import { defineLoader } from 'vitepress'

import { discoverRuleIndex } from '../lib/rules/discovery'
import { readPipeline }      from '../lib/rules/pipeline'
import * as paths            from '../lib/shared/paths'
import * as registries       from '../lib/shared/registries'

interface PipelineRule {
  category      : registries.RuleCategory | null
  categoryBadge : string | null
  categoryLabel : string | null
  documented    : boolean
  family        : registries.RuleFamily | null
  familyBadge   : string | null
  familyLabel   : string | null
  imperative    : string
  position      : number
  slug          : string
}

interface PipelineData {
  rules : readonly PipelineRule[]
}

const rulesDirectory = paths.rulesDir(import.meta.url)

declare const data: PipelineData
export { data }

export default defineLoader({
  watch: [...paths.proseBinaryCandidates(paths.repoRoot(import.meta.url)), `${rulesDirectory}/*.md`],
  load(): PipelineData {
    const discovered = discoverRuleIndex(rulesDirectory)
    const rules      = readPipeline(import.meta.url).map(({ imperative, position, slug }) => {
      const entry        = discovered.get(slug)
      const categoryMeta = entry ? registries.CATEGORY_META[entry.category] : null
      const familyMeta   = entry ? registries.FAMILY_META[entry.family]     : null
      return {
        category      : entry?.category ?? null,
        categoryBadge : categoryMeta?.badge ?? null,
        categoryLabel : categoryMeta?.label ?? null,
        documented    : entry !== undefined,
        family        : entry?.family ?? null,
        familyBadge   : familyMeta?.badge ?? null,
        familyLabel   : familyMeta?.label ?? null,
        imperative,
        position,
        slug
      }
    })
    return { rules }
  }
})
