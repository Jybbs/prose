import { defineLoader } from 'vitepress'

import { getRenderer, renderPlainInlineHtml } from '../markdown/renderer'
import { rulesDir }                           from '../shared/paths'
import { FAMILY_META }                        from '../shared/registries'
import { discoverRuleSlugs }                  from './discovery'
import { groupRules }                         from './grouping'
import type { RenderedRule, RulesData }       from './grouping'

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
