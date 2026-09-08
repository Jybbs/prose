import { defineLoader } from 'vitepress'

import type { InlineNode }              from '../markdown/inline-nodes'
import { getRenderer, inlineNodeField } from '../markdown/renderer'
import { discoverRuleSlugs }            from '../rules/discovery'
import { rulesDir }                     from '../shared/paths'
import { LINT_CODE, SOURCES }           from './sources'

interface ExitCode {
  code        : number
  detailNodes : InlineNode[][]
  label       : string
  summary     : string
}

declare const data: readonly ExitCode[]
export { data }

const rulesDirectory = rulesDir(import.meta.url)

export default defineLoader({
  watch: [`${rulesDirectory}/*/*.md`],
  async load(): Promise<readonly ExitCode[]> {
    const md     = await getRenderer()
    const roster = discoverRuleSlugs(rulesDirectory)
      .filter(rule => rule.lints)
      .map(rule => `\`${rule.slug}\``)
      .join(', ')
    const sources = SOURCES.map(source => source.code === LINT_CODE
      ? { ...source, detail: [...source.detail, `The shipped lints can produce it: ${roster}.`] }
      : source)
    return inlineNodeField(md, sources, 'detail')
  }
})
