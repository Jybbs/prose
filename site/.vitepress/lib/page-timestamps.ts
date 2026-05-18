import { execFileSync } from 'node:child_process'

const SITE_PREFIX = 'site/'
const MD_SUFFIX   = '.md'

export function buildPageTimestamps(repoRoot: string): Map<string, number> {
  let raw: string
  try {
    raw = execFileSync(
      'git',
      ['log', '--name-only', '--pretty=format:%H|%aI', '--diff-filter=AM', '--', 'site/'],
      { cwd: repoRoot, encoding: 'utf8', maxBuffer: 16 * 1024 * 1024 }
    )
  }
  catch (err) {
    console.warn('[config:page-timestamps] git log failed, falling back to empty:', err instanceof Error ? err.message : err)
    return new Map()
  }

  const out = new Map<string, number>()
  let   currentMs = 0
  for (const line of raw.split('\n')) {
    if (!line) continue
    if (line.includes('|')) {
      currentMs = Date.parse(line.split('|')[1])
      continue
    }
    if (!line.startsWith(SITE_PREFIX) || !line.endsWith(MD_SUFFIX)) continue
    const relative = line.slice(SITE_PREFIX.length)
    if (!out.has(relative)) out.set(relative, currentMs)
  }
  return out
}
