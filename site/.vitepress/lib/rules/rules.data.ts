import { defineLoader } from 'vitepress'

import { getRenderer, renderPlainInlineHtml } from '../markdown/renderer.ts'
import { rulesDir }                           from '../shared/paths.ts'
import { FAMILY_META }                        from '../shared/registries.ts'
import { discoverRuleSlugs }                  from './discovery.ts'
import { groupRules }                         from './grouping.ts'
import type { RenderedRule, RulesData }       from './grouping.ts'

export type { RenderedRule, RulesData }

const rulesDirectory = rulesDir(import.meta.url)

declare const data: RulesData
export { data }

export default defineLoader({
  watch: [`${rulesDirectory}/*/*.md`],
  async load(): Promise<RulesData> {
    const md         = await getRenderer()
    const list       = discoverRuleSlugs(rulesDirectory).map(r => ({
      ...r,
      captionHtml : renderPlainInlineHtml(md, r.caption),
      familyLabel : FAMILY_META[r.family].label
    }))
    return groupRules(list)
  }
})
