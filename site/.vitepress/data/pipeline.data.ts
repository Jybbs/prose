import fs   from 'node:fs'
import path from 'node:path'

import { defineLoader } from 'vitepress'

import { discoverRuleSlugs }              from '../lib/rules/discovery'
import { repoRoot, rulesDir }             from '../lib/shared/paths'
import type { RuleCategory, RuleFamily }  from '../lib/shared/registries'

export interface PipelineRule {
  category   : RuleCategory | null
  documented : boolean
  family     : RuleFamily | null
  imperative : string
  position   : number
  slug       : string
}

export interface PipelineData {
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
    const text  = fs.readFileSync(ruleSource, 'utf8')
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
        category   : entry?.category ?? null,
        documented : entry !== undefined,
        family     : entry?.family ?? null,
        imperative,
        position   : rules.length + 1,
        slug
      })
    }
    if (rules.length === 0) {
      throw new Error(`pipeline.data: register_rules! parsed zero rules from ${ruleSource}`)
    }
    return { rules }
  }
})
