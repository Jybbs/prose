import fs   from 'node:fs/promises'
import path from 'node:path'

import { defineLoader } from 'vitepress'

import { discoverRuleSlugs }                                       from '../lib/rules/discovery'
import { repoRoot, rulesDir }                                      from '../lib/shared/paths'
import { CATEGORY_META, FAMILY_META, type RuleCategory, type RuleFamily } from '../lib/shared/registries'

interface PipelineRule {
  category      : RuleCategory | null
  categoryBadge : string | null
  categoryLabel : string | null
  documented    : boolean
  family        : RuleFamily | null
  familyBadge   : string | null
  familyLabel   : string | null
  imperative    : string
  position      : number
  slug          : string
}

interface PipelineData {
  rules : readonly PipelineRule[]
}

const REGISTER_BLOCK = /register_rules!\s*{([\s\S]*?)\n}/
const RULE_LINE      = /^\s*"([a-z][a-z0-9-]*)"\s*:\s*\w+\s*:\s*\w+\s*=>\s*\w+\s*=>\s*"([^"]+)"\s*,?\s*$/

const repoDir        = repoRoot(import.meta.url)
const ruleSource     = path.join(repoDir, 'src', 'rule.rs')
const rulesDirectory = rulesDir(import.meta.url)

declare const data: PipelineData
export { data }

export default defineLoader({
  watch: [ruleSource, `${rulesDirectory}/*.md`],
  async load(): Promise<PipelineData> {
    const text  = await fs.readFile(ruleSource, 'utf8')
    const block = REGISTER_BLOCK.exec(text)
    if (block === null) {
      throw new Error(`pipeline.data: register_rules! block not found in ${ruleSource}`)
    }
    const discovered = new Map(discoverRuleSlugs(rulesDirectory).map(r => [r.slug, r]))
    const rules: PipelineRule[] = []
    for (const line of block[1].split('\n')) {
      const match = RULE_LINE.exec(line)
      if (match === null) continue
      const slug       = match[1]
      const imperative = match[2]
      const entry      = discovered.get(slug)
      rules.push({
        category      : entry?.category ?? null,
        categoryBadge : entry ? CATEGORY_META[entry.category].badge : null,
        categoryLabel : entry ? CATEGORY_META[entry.category].label : null,
        documented    : entry !== undefined,
        family        : entry?.family ?? null,
        familyBadge   : entry ? FAMILY_META[entry.family].badge : null,
        familyLabel   : entry ? FAMILY_META[entry.family].label : null,
        imperative,
        position      : rules.length + 1,
        slug
      })
    }
    if (rules.length === 0) {
      throw new Error(`pipeline.data: register_rules! parsed zero rules from ${ruleSource}`)
    }
    return { rules }
  }
})
