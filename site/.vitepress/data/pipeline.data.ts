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
  async load(): Promise<PipelineData> {
    const discovered = discoverRuleIndex(rulesDirectory)
    const rules      = readPipeline(import.meta.url).map(({ imperative, position, slug }) => {
      const entry = discovered.get(slug)
      return {
        category      : entry?.category ?? null,
        categoryBadge : entry ? registries.CATEGORY_META[entry.category].badge : null,
        categoryLabel : entry ? registries.CATEGORY_META[entry.category].label : null,
        documented    : entry !== undefined,
        family        : entry?.family ?? null,
        familyBadge   : entry ? registries.FAMILY_META[entry.family].badge : null,
        familyLabel   : entry ? registries.FAMILY_META[entry.family].label : null,
        imperative,
        position,
        slug
      }
    })
    return { rules }
  }
})
