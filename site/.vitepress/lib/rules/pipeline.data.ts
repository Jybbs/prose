import { defineLoader } from 'vitepress'

import { discoverRuleIndex } from './discovery'
import { readPipeline }      from './pipeline'
import * as paths            from '../shared/paths'
import * as registries       from '../shared/registries'

interface PipelineRule {
  after         : readonly string[]
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
    const rules      = readPipeline(import.meta.url).map(({ after, imperative, position, slug }) => {
      const entry        = discovered.get(slug)
      const categoryMeta = entry ? registries.CATEGORY_META[entry.category] : null
      const familyMeta   = entry ? registries.FAMILY_META[entry.family]     : null
      return {
        after,
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
