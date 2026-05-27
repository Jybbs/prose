import fs   from 'node:fs'
import path from 'node:path'

import { repoRoot } from '../shared/paths'

interface PipelineEntry {
  imperative : string
  position   : number
  slug       : string
}

const REGISTER_BLOCK = /register_rules!\s*{([\s\S]*?)\n}/
const RULE_LINE      = /^\s*"([a-z][a-z0-9-]*)"\s*:\s*\w+\s*:\s*\w+\s*=>\s*\w+\s*=>\s*"([^"]+)"\s*,?\s*$/

export function ruleSourcePath(metaUrl: string): string {
  return path.join(repoRoot(metaUrl), 'src', 'rule.rs')
}

export function parsePipeline(metaUrl: string): readonly PipelineEntry[] {
  const ruleSource = ruleSourcePath(metaUrl)
  const text       = fs.readFileSync(ruleSource, 'utf8')
  const block      = REGISTER_BLOCK.exec(text)
  if (block === null) {
    throw new Error(`parsePipeline: register_rules! block not found in ${ruleSource}`)
  }
  const matches = block[1].split('\n').map(line => RULE_LINE.exec(line)).filter(m => m !== null)
  if (matches.length === 0) {
    throw new Error(`parsePipeline: register_rules! parsed zero rules from ${ruleSource}`)
  }
  return matches.map((match, i) => ({ imperative: match[2], position: i + 1, slug: match[1] }))
}
