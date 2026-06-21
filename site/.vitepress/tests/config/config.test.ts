import { canonicalUrl }                          from '../../lib/config/canonical-url'
import { ogImagePath, ogImageUrl }               from '../../lib/config/og-url'
import { buildPageTimestamps, parseGitTimestamps } from '../../lib/config/page-timestamps'
import { SITE_HOSTNAME }                         from '../../lib/shared/constants'
import { repoRoot }                              from '../../lib/shared/paths'
import { warnTest }                              from '../support'

describe('canonicalUrl', () => {
  it.each([
    ['index.md',                 `${SITE_HOSTNAME}/`],
    ['reference/cli.md',         `${SITE_HOSTNAME}/reference/cli`],
    ['rules/alignment/index.md', `${SITE_HOSTNAME}/rules/alignment/`]
  ])('maps %s', (rel, expected) => {
    expect(canonicalUrl(rel)).toBe(expected)
  })
})

describe('ogImagePath', () => {
  it.each([
    ['index.md',                        'og.png'],
    ['reference/cli.md',                'og/reference/cli.png'],
    ['rules/alignment/align-equals.md', 'og/rules/alignment/align-equals.png']
  ])('maps %s', (rel, expected) => {
    expect(ogImagePath(rel)).toBe(expected)
  })
})

describe('ogImageUrl', () => {
  it('prefixes the hostname onto the card path', () => {
    expect(ogImageUrl('reference/cli.md')).toBe(`${SITE_HOSTNAME}/og/reference/cli.png`)
  })
})

describe('parseGitTimestamps', () => {
  it('keeps the newest timestamp per site markdown file and skips the rest', () => {
    const raw = [
      '2024-02-20T12:00:00+00:00', '', 'site/reference/cli.md',
      '2024-01-15T10:00:00+00:00', '', 'site/reference/cli.md', 'site/usage/quick-start.md', 'site/notes.txt'
    ].join('\n')
    const map = parseGitTimestamps(raw)
    expect(map.get('reference/cli.md')).toBe(Date.parse('2024-02-20T12:00:00+00:00'))
    expect(map.get('usage/quick-start.md')).toBe(Date.parse('2024-01-15T10:00:00+00:00'))
    expect(map.has('notes.txt')).toBe(false)
  })
})

describe('buildPageTimestamps', () => {
  it('reads the repo history into a map', () => {
    const map = buildPageTimestamps(repoRoot(import.meta.url))
    expect(map).toBeInstanceOf(Map)
    expect(map.size).toBeGreaterThan(0)
  })

  warnTest('falls back to an empty map and warns when git fails', ({ warn }) => {
    expect(buildPageTimestamps('/no/such/repo/here')).toEqual(new Map())
    expect(warn).toHaveBeenCalledOnce()
  })
})
