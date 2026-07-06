import { canonicalUrl }                          from '../../lib/config/canonical-url'
import { articleLd, landingLd }                  from '../../lib/config/json-ld'
import { ogImagePath, ogImageUrl }               from '../../lib/config/og-url'
import { buildPageTimestamps, parseGitTimestamps } from '../../lib/config/page-timestamps'
import { ROBOTS_TXT }                            from '../../lib/config/robots'
import { attachLastmod }                         from '../../lib/config/sitemap'
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

describe('articleLd', () => {
  it('emits a TechArticle carrying the schema context and the page facts', () => {
    const facts = { description: 'd', headline: 'Align Equals', image: 'i.png', url: 'u' }
    expect(JSON.parse(articleLd(facts))).toEqual({
      '@context' : 'https://schema.org',
      '@type'    : 'TechArticle',
      ...facts
    })
  })
})

describe('landingLd', () => {
  it('emits a WebSite and SoftwareApplication graph pinned to the version', () => {
    const url = `${SITE_HOSTNAME}/`
    const { '@context': context, '@graph': graph } = JSON.parse(landingLd(url, '0.1.0', 'd'))
    expect(context).toBe('https://schema.org')
    expect(graph).toMatchObject([
      { '@type': 'WebSite',             description: 'd',         name: 'Prose', url },
      { '@type': 'SoftwareApplication', softwareVersion: '0.1.0', name: 'Prose', url }
    ])
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

describe('ROBOTS_TXT', () => {
  it('allows every crawler and points at the emitted sitemap', () => {
    expect(ROBOTS_TXT.split('\n')).toEqual([
      'User-agent: *',
      'Allow: /',
      '',
      `Sitemap: ${SITE_HOSTNAME}/sitemap.xml`,
      ''
    ])
  })
})

describe('attachLastmod', () => {
  const timestamps = new Map([
    ['index.md',                 1000],
    ['reference/cli.md',         2000],
    ['rules/alignment/index.md', 3000]
  ])

  it.each([
    ['',                   1000],
    ['reference/cli',      2000],
    ['reference/cli.html', 2000],
    ['rules/alignment/',   3000]
  ])('maps the sitemap url %j back to its page timestamp', (url, lastmod) => {
    expect(attachLastmod([{ url }], timestamps)).toEqual([{ lastmod, url }])
  })

  it('leaves an item without a timestamp untouched', () => {
    expect(attachLastmod([{ url: 'ghost' }], timestamps)).toEqual([{ url: 'ghost' }])
  })
})
