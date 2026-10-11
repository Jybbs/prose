import fs   from 'node:fs'
import path from 'node:path'

import matter from 'gray-matter'

import * as head                                 from '../../lib/config/head'
import { SITE_ALT, SITE_HOSTNAME, SITE_TAGLINE } from '../../lib/shared/constants'
import { siteDir }                               from '../../lib/shared/paths'

import type { HeadConfig, PageData } from 'vitepress'

function page(overrides: Partial<PageData>): PageData {
  return { description: '', frontmatter: {}, relativePath: '', title: '', ...overrides } as PageData
}

function content(tags: HeadConfig[], key: string): string | undefined {
  const entry = tags.find(([, attrs]) => attrs.property === key || attrs.name === key)
  return entry?.[1].content
}

function jsonLd(tags: HeadConfig[]): unknown {
  const entry = tags.find(([tag]) => tag === 'script')
  return entry === undefined ? undefined : JSON.parse(entry[2] as string)
}

describe('pageHead', () => {
  it('emits the article shape for a content page', () => {
    const data = page({
      description  : 'd',
      relativePath : 'usage/quick-start.md',
      title        : 'Quick Start'
    })
    const tags = head.pageHead(data, '0.1.0')
    expect(content(tags, 'og:title')).toBe('Quick Start')
    expect(content(tags, 'og:type')).toBe('article')
    expect(content(tags, 'og:url')).toBe(`${SITE_HOSTNAME}/usage/quick-start`)
    expect(content(tags, 'og:image')).toBe(`${SITE_HOSTNAME}/og/usage/quick-start.png`)
    expect(jsonLd(tags)).toMatchObject({ '@type': 'TechArticle', headline: 'Quick Start' })
  })

  it('emits the website graph for the landing', () => {
    const data = page({ description: 'd', relativePath: 'index.md', title: 'Home' })
    const tags = head.pageHead(data, '0.1.0')
    expect(content(tags, 'og:title')).toBe('Prose')
    expect(content(tags, 'og:type')).toBe('website')
    expect(content(tags, 'og:image')).toBe(`${SITE_HOSTNAME}/og.png`)
    expect(jsonLd(tags)).toMatchObject({ '@graph': [
      { '@type': 'WebSite' },
      { '@type': 'SoftwareApplication', softwareVersion: '0.1.0' }
    ] })
  })

  it('falls back to the landing card and drops og:url and JSON-LD on the 404', () => {
    const data = page({
      description  : 'Not Found',
      isNotFound   : true,
      relativePath : '404.md',
      title        : '404'
    })
    const tags = head.pageHead(data, '0.1.0')
    expect(content(tags, 'og:title')).toBe('Prose')
    expect(content(tags, 'og:type')).toBe('website')
    expect(content(tags, 'og:image')).toBe(`${SITE_HOSTNAME}/og.png`)
    expect(content(tags, 'og:description')).toBe(SITE_TAGLINE)
    expect(content(tags, 'og:url')).toBeUndefined()
    expect(jsonLd(tags)).toBeUndefined()
  })

  it('prefers the frontmatter name over the page title', () => {
    const data = page({
      frontmatter  : { name: 'Align Equals' },
      relativePath : 'rules/alignment/align-equals.md',
      title        : 'align-equals'
    })
    expect(content(head.pageHead(data, '0.1.0'), 'og:title')).toBe('Align Equals')
  })
})

describe('describePage', () => {
  it('flattens the description field to the text a reader sees', () => {
    const data = page({
      frontmatter: { description: 'Covers `prose check` and *Prose* **itself**.' }
    })
    head.describePage(data)
    expect(data.description).toBe('Covers prose check and Prose itself.')
  })

  it.each([
    [{ caption: 'c', summary: 's' },                   'c'],
    [{ summary: 's', caption: 'c' },                   'c'],
    [{ description: 'd', caption: 'c', summary: 's' }, 'd'],
    [{ description: ['d'], caption: 'c' },             'c'],
    [{ summary: '`s`' },                               's']
  ])('takes the first string in DESCRIPTION_FIELDS order from %o', (frontmatter, expected) => {
    const data = page({ frontmatter })
    head.describePage(data)
    expect(data.description).toBe(expected)
  })

  it('leaves the description of a page declaring no field alone', () => {
    const data = page({ relativePath: 'reference/cli.md' })
    head.describePage(data)
    expect(data.description).toBe('')
  })

  it('titles the landing page with the site name and tagline and drops the template', () => {
    const data = page({ relativePath: 'index.md' })
    head.describePage(data)
    expect(data).toMatchObject({ title: SITE_ALT, titleTemplate: false })
  })

  it('leaves every other page on the site title template', () => {
    const data = page({ relativePath: 'usage/quick-start.md', title: 'Quick Start' })
    head.describePage(data)
    expect(data).toMatchObject({ title: 'Quick Start' })
    expect(data.titleTemplate).toBeUndefined()
  })
})

describe('every page but the landing', () => {
  const site  = siteDir(import.meta.url)
  const pages = fs.globSync('**/*.md', { cwd: site, exclude: ['node_modules/**'] })
    .filter(file => file !== 'index.md')

  it.each(pages)('%s declares a description field', file => {
    const { data } = matter.read(path.join(site, file))
    expect(head.DESCRIPTION_FIELDS.some(field => typeof data[field] === 'string')).toBe(true)
  })
})
