import path from 'node:path'

import { defineLoader } from 'vitepress'

import { repoRoot }                          from '../lib/paths'
import { discoverRules, type DiscoveredRule } from '../lib/rules'

const rulesDir = path.join(repoRoot(import.meta.url), 'site/rules')

declare const data: DiscoveredRule[]
export { data }

export default defineLoader({
  watch: [`${rulesDir}/*.md`],
  load(): DiscoveredRule[] {
    return discoverRules(rulesDir)
  }
})
