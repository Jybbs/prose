import { execFileSync } from 'node:child_process'

import { repoRoot } from '../shared/paths'

const CONTENT_PREFIX = 'site/src/content/docs/'
const MD_SUFFIX      = '.md'

// Maps each docs page to the ISO date of its most recent add-or-modify commit,
// read in one `git log` pass for the sitemap `lastmod`. A git failure yields an
// empty map, leaving entries without a `lastmod`.
export function buildContentTimestamps(siteRoot: URL): Map<string, string> {
  try {
    const raw = execFileSync(
      'git',
      ['log', '--name-only', '--pretty=format:%aI', '--diff-filter=AM', '--', CONTENT_PREFIX],
      { cwd: repoRoot(siteRoot), encoding: 'utf8', maxBuffer: 16 * 1024 * 1024 }
    )
    return parseContentTimestamps(raw)
  } catch (err) {
    console.warn('[sitemap] git log failed, omitting lastmod:', err instanceof Error ? err.message : err)
    return new Map()
  }
}

export function parseContentTimestamps(raw: string): Map<string, string> {
  const out = new Map<string, string>()
  let   isoDate = ''
  for (const line of raw.split('\n')) {
    if (!line) continue
    if (!line.startsWith(CONTENT_PREFIX)) {
      isoDate = line
      continue
    }
    if (!line.endsWith(MD_SUFFIX)) continue
    const slug = line.slice(CONTENT_PREFIX.length)
    if (!out.has(slug)) out.set(slug, isoDate)
  }
  return out
}

// Resolves a sitemap item URL to the ISO date of its source page, trying the
// page file and its index variant.
export function lastmodForUrl(url: string, timestamps: Map<string, string>): string | undefined {
  const slug       = new URL(url).pathname.split('/').filter(Boolean).join('/')
  const candidates = slug === '' ? ['index.md'] : [`${slug}.md`, `${slug}/index.md`]
  return candidates.map(candidate => timestamps.get(candidate)).find(Boolean)
}
