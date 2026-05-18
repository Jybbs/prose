import { defineLoader } from 'vitepress'

import type { RuleCategory } from '../lib/categories'
import { rulesDir }          from '../lib/paths'
import { discoverRuleFiles } from '../lib/rules-discovery'

export type { RuleCategory }

export interface DiscoveredRule {
  category : RuleCategory
  slug     : string
}

declare const data: DiscoveredRule[]
export { data }

export default defineLoader({
  watch: [`${rulesDir(import.meta.url)}/*.md`],
  load(): DiscoveredRule[] {
    return discoverRuleFiles(rulesDir(import.meta.url))
      .map(({ category, slug }) => ({ category, slug }))
      .sort((a, b) => a.slug.localeCompare(b.slug))
  }
})
