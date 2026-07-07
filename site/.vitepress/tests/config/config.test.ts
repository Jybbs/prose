import { canonicalUrl }            from '../../lib/config/canonical-url'
import { articleLd, landingLd }    from '../../lib/config/json-ld'
import { ogImagePath, ogImageUrl } from '../../lib/config/og-url'
import { ROBOTS_TXT }              from '../../lib/config/robots'
import { SITE_HOSTNAME }           from '../../lib/shared/constants'

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
