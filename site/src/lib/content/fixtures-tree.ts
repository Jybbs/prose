import { readdirSync } from 'node:fs'
import path            from 'node:path'

import { parseFrontmatter } from '@astrojs/markdown-remark'

// The `<rule>/<case>` id, the rule slug in kebab form so it joins the docs
// collection's rule slugs.
export const fixtureId = (rule: string, name: string): string => `${rule.replaceAll('_', '-')}/${name}`

const subdirectories = (dir: string): string[] =>
  readdirSync(dir, { withFileTypes: true }).filter(e => e.isDirectory()).map(e => e.name).sort()

// Each `<rule>/<case>` case directory under the fixtures root, the rule and case
// names alongside the joined path.
export function* fixtureDirs(root: string): Iterable<{ dir: string, name: string, rule: string }> {
  for (const rule of subdirectories(root)) {
    const ruleDir = path.join(root, rule)
    for (const name of subdirectories(ruleDir)) yield { dir: path.join(ruleDir, name), name, rule }
  }
}

// Drops the insta YAML frontmatter the snapshot tooling writes, leaving the
// recorded body the source-of-truth output. `parseFrontmatter` removes the
// frontmatter block, and the slice drops the newline it leaves before the body.
export function snapshotBody(raw: string): string {
  const content = parseFrontmatter(raw).content
  return content.startsWith('\n') ? content.slice(1) : content
}
