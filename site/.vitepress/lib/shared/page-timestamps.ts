import { execFileSync } from 'node:child_process'

import { withFallbackSync } from './with-fallback'

const SITE_PREFIX = 'site/'
const MD_SUFFIX   = '.md'

export function buildPageTimestamps(repoRoot: string): Map<string, number> {
  return withFallbackSync('page-timestamps:git-log', () => {
    const raw = execFileSync(
      'git',
      ['log', '--name-only', '--pretty=format:%aI', '--diff-filter=AM', '--', 'site/'],
      { cwd: repoRoot, encoding: 'utf8', maxBuffer: 16 * 1024 * 1024 }
    )
    const out = new Map<string, number>()
    let   currentMs = 0
    for (const line of raw.split('\n')) {
      if (!line) continue
      if (!line.startsWith(SITE_PREFIX)) {
        currentMs = Date.parse(line)
        continue
      }
      if (!line.endsWith(MD_SUFFIX)) continue
      const relative = line.slice(SITE_PREFIX.length)
      if (!out.has(relative)) out.set(relative, currentMs)
    }
    return out
  }, new Map())
}
