import fs   from 'node:fs'
import path from 'node:path'

import type { RuleCategory } from './categories'

export interface DiscoveredRuleFile {
  category : RuleCategory
  related  : readonly string[]
  slug     : string
}

export function discoverRuleFiles(rulesDirectory: string): DiscoveredRuleFile[] {
  const out: DiscoveredRuleFile[] = []
  for (const file of fs.readdirSync(rulesDirectory).sort()) {
    if (!file.endsWith('.md') || file === 'index.md') continue
    const body     = fs.readFileSync(path.join(rulesDirectory, file), 'utf8')
    const front    = body.match(/^---\n([\s\S]*?)\n---\n/)?.[1] ?? ''
    const slug     = file.slice(0, -'.md'.length)
    const category = parseCategory(front, slug)
    const related  = parseRelated(front)
    out.push({ category, related, slug })
  }
  validateRelated(out)
  return out
}

export function splitByCategory(rules: readonly DiscoveredRuleFile[]): { autoFix: string[]; lint: string[] } {
  const autoFix: string[] = []
  const lint   : string[] = []
  for (const r of rules) {
    if (r.category === 'lint') lint.push(r.slug)
    else                       autoFix.push(r.slug)
  }
  return { autoFix, lint }
}

function parseCategory(frontmatter: string, slug: string): RuleCategory {
  const m = frontmatter.match(/^\s*category:\s*(\S+)\s*$/m)
  const v = m?.[1]
  if (v === 'auto-fix' || v === 'lint') return v
  throw new Error(`Rule "${slug}" has invalid or missing category: ${JSON.stringify(v)}`)
}

function parseRelated(frontmatter: string): string[] {
  const m = frontmatter.match(/^\s*related:\s*\[([^\]]*)\]/m)
  if (!m) return []
  return m[1]
    .split(',')
    .map(s => s.trim().replace(/^["']|["']$/g, ''))
    .filter(Boolean)
}

function validateRelated(rules: readonly DiscoveredRuleFile[]): void {
  const slugs = new Set(rules.map(r => r.slug))
  for (const r of rules) {
    for (const ref of r.related) {
      if (!slugs.has(ref)) {
        throw new Error(`Rule "${r.slug}" lists invalid related slug "${ref}"`)
      }
    }
  }
}
