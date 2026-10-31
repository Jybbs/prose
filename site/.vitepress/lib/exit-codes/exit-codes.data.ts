import { defineLoader } from 'vitepress'

import type { InlineNode }                     from '../markdown/inline-nodes'
import { getRenderer, inlineNodeField }        from '../markdown/renderer'
import { discoverRuleSlugs }                   from '../rules/discovery'
import { proseBinaryPath, repoRoot, rulesDir } from '../shared/paths'
import { LINT_CODE, exitCodes }                from './sources'

interface ExitCode {
  code        : number
  detailNodes : InlineNode[][]
  label       : string
  summary     : string
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
    const sources = exitCodes(root).map(source => source.code === LINT_CODE
      ? { ...source, detail: [...source.detail, `The shipped lints can produce it: ${roster}.`] }
      : source)
    return inlineNodeField(md, sources, 'detail')
  }
})
