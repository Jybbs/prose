import { pageHead }                    from '../../lib/config/head'
import { SITE_HOSTNAME, SITE_TAGLINE } from '../../lib/shared/constants'

import type { HeadConfig, PageData } from 'vitepress'

function page(overrides: Partial<PageData>): PageData {
  return { description: '', frontmatter: {}, relativePath: '', title: '', ...overrides } as PageData
}

function content(head: HeadConfig[], key: string): string | undefined {
  const entry = head.find(([, attrs]) => attrs.property === key || attrs.name === key)
  return entry?.[1].content
}

function jsonLd(head: HeadConfig[]): unknown {
  const entry = head.find(([tag]) => tag === 'script')
  return entry === undefined ? undefined : JSON.parse(entry[2] as string)
}

describe('pageHead', () => {
  it('emits the article shape for a content page', () => {
    const data = page({ description: 'd', relativePath: 'usage/quick-start.md', title: 'Quick Start' })
    const head = pageHead(data, '0.1.0')
    expect(content(head, 'og:title')).toBe('Quick Start · Prose')
    expect(content(head, 'og:type')).toBe('article')
    expect(content(head, 'og:url')).toBe(`${SITE_HOSTNAME}/usage/quick-start`)
    expect(content(head, 'og:image')).toBe(`${SITE_HOSTNAME}/og/usage/quick-start.png`)
    expect(jsonLd(head)).toMatchObject({ '@type': 'TechArticle', headline: 'Quick Start' })
  })

  it('emits the website graph for the landing', () => {
    const head = pageHead(page({ description: 'd', relativePath: 'index.md', title: 'Home' }), '0.1.0')
    expect(content(head, 'og:title')).toBe('Prose')
    expect(content(head, 'og:type')).toBe('website')
    expect(content(head, 'og:image')).toBe(`${SITE_HOSTNAME}/og.png`)
    expect(jsonLd(head)).toMatchObject({ '@graph': [
      { '@type': 'WebSite' },
      { '@type': 'SoftwareApplication', softwareVersion: '0.1.0' }
    ] })
  })

  it('falls back to the landing card and drops og:url and JSON-LD on the 404', () => {
    const data = page({ description: 'Not Found', isNotFound: true, relativePath: '404.md', title: '404' })
    const head = pageHead(data, '0.1.0')
    expect(content(head, 'og:type')).toBe('website')
    expect(content(head, 'og:image')).toBe(`${SITE_HOSTNAME}/og.png`)
    expect(content(head, 'og:description')).toBe(SITE_TAGLINE)
    expect(content(head, 'og:url')).toBeUndefined()
    expect(jsonLd(head)).toBeUndefined()
  })

  it('prefers the frontmatter name over the page title', () => {
    const data = page({
      frontmatter  : { name: 'Align Equals' },
      relativePath : 'rules/alignment/align-equals.md',
      title        : 'align-equals'
    })
    expect(content(pageHead(data, '0.1.0'), 'og:title')).toBe('Align Equals · Prose')
  })
})
