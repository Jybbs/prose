import { defineLoader } from 'vitepress'

import type { InlineNode }                     from '../markdown/inline-nodes.ts'
import { getRenderer, inlineNodeField }        from '../markdown/renderer.ts'
import { discoverRuleSlugs }                   from '../rules/discovery.ts'
import { proseBinaryPath, repoRoot, rulesDir } from '../shared/paths.ts'
import * as sources                            from './sources.ts'

interface ExitCode {
  band    : string
  code    : number
  facts   : readonly { label: string, nodes: InlineNode[] }[]
  label   : string
  outcome : sources.Outcome
  summary : string
}

declare const data: readonly ExitCode[]
export { data }

const root           = repoRoot(import.meta.url)
const rulesDirectory = rulesDir(import.meta.url)

export default defineLoader({
  watch: [`${rulesDirectory}/*/*.md`, proseBinaryPath(root)],
  async load(): Promise<readonly ExitCode[]> {
    const md     = await getRenderer()
    const roster = discoverRuleSlugs(rulesDirectory)
      .filter(rule => rule.lints)
      .map(rule => `\`${rule.slug}\``)
      .join(', ')
    const entries = sources.exitCodes(root).map(source => {
      if (source.code !== sources.LINT_CODE) return source
      const [returned, gate, next, note] = source.facts
      return { ...source, facts: [returned, gate, next, `${note} ${roster}.`] as const }
    })
    return inlineNodeField(md, entries, 'facts').map(({ factsNodes, ...entry }) => ({
      ...entry,
      band  : sources.OUTCOME_LABELS[entry.outcome],
      facts : factsNodes.map((nodes, at) => ({ label: sources.FACT_LABELS[at]!, nodes }))
    }))
  }
})
