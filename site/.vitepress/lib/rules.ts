import fs   from 'node:fs'
import path from 'node:path'

export type RuleCategory = 'auto-fix' | 'lint'

export interface DiscoveredRule {
  category : RuleCategory
  slug     : string
}

export function discoverRules(rulesDir: string): DiscoveredRule[] {
  const out: DiscoveredRule[] = []
  for (const file of fs.readdirSync(rulesDir).sort()) {
    if (!file.endsWith('.md') || file === 'index.md') continue
    const body  = fs.readFileSync(path.join(rulesDir, file), 'utf8')
    const front = body.match(/^---\n([\s\S]*?)\n---\n/)?.[1] ?? ''
    out.push({
      category : /^\s*category:\s*lint\s*$/m.test(front) ? 'lint' : 'auto-fix',
      slug     : file.slice(0, -'.md'.length)
    })
  }
  return out
}
