import { defineLoader } from 'vitepress'

import { discoverRuleIndex } from './discovery'
import { readPipeline }      from './pipeline'
import * as paths            from '../shared/paths'
import type { RuleFamily }   from '../shared/registries'

interface PipelineRule {
  documented    : boolean
  family        : RuleFamily | null
  position      : number
  preservesTree : boolean
  slug          : string
  title         : string
}

interface PipelineData {
  rules : readonly PipelineRule[]
}

const rulesDirectory = paths.rulesDir(import.meta.url)

declare const data: PipelineData
export { data }

export default defineLoader({
  watch: [paths.proseBinaryPath(paths.repoRoot(import.meta.url)), `${rulesDirectory}/*/*.md`],
  load(): PipelineData {
    const discovered = discoverRuleIndex(rulesDirectory)
    const rules      = readPipeline(import.meta.url).map(({ after, position, preserves_tree, slug }) => {
      const entry  = discovered.get(slug)
      const behind = after.length > 0 ? ` · runs behind ${after.join(', ')}` : ''
      return {
        documented    : entry !== undefined,
        family        : entry?.family ?? null,
        position      : position,
        preservesTree : preserves_tree,
        slug          : slug,
        title         : `${slug}${entry ? ` (${entry.family})` : ''}${behind}`
      }
    })
    return { rules }
  }
})
