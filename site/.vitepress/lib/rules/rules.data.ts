import { defineLoader } from 'vitepress'

import { getRenderer, renderPlainInlineHtml } from '../markdown/renderer'
import { rulesDir }                          from '../shared/paths'
import * as registries                       from '../shared/registries'
import { toTitleCase }                       from '../shared/title-case'
import { discoverRuleSlugs }                 from './discovery'
import { groupRules }                        from './grouping'
import type { RenderedRule, RulesData }      from './grouping'

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
      captionHtml   : renderPlainInlineHtml(md, r.caption),
      categoryBadge : registries.CATEGORY_META[r.category].badge,
      categoryLabel : registries.CATEGORY_META[r.category].label,
      familyBadge   : registries.FAMILY_META[r.family].badge,
      familyLabel   : registries.FAMILY_META[r.family].label,
      name          : toTitleCase(r.slug, '-')
    }))
    return groupRules(list)
  }
})
