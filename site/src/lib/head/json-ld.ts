import type { Graph, TechArticle, WithContext } from 'schema-dts'

const CONTEXT = 'https://schema.org'

interface ArticleFacts {
  description ?: string
  headline     : string
  image        : string
  url          : string
}

export function articleLd(facts: ArticleFacts): string {
  const article: WithContext<TechArticle> = {
    '@context' : CONTEXT,
    '@type'    : 'TechArticle',
    ...facts
  }
  return JSON.stringify(article)
}

export function landingLd(site: string, version: string, description?: string): string {
  const graph: Graph = {
    '@context' : CONTEXT,
    '@graph'   : [
      { '@type': 'WebSite', description, name: 'Prose', url: site },
      {
        '@type'             : 'SoftwareApplication',
        applicationCategory : 'DeveloperApplication',
        license             : 'https://opensource.org/license/mit',
        name                : 'Prose',
        operatingSystem     : 'Linux, macOS, Windows',
        softwareVersion     : version,
        url                 : site
      }
    ]
  }
  return JSON.stringify(graph)
}

if (import.meta.vitest) {
  const { describe, expect, test } = import.meta.vitest

  describe('articleLd', () => {
    test('emits a TechArticle with the schema context and facts', () => {
      const parsed = JSON.parse(articleLd({ description: 'd', headline: 'Align Equals', image: 'i.png', url: 'u' }))
      expect(parsed).toMatchObject({
        '@context' : 'https://schema.org',
        '@type'    : 'TechArticle',
        description: 'd',
        headline   : 'Align Equals',
        image      : 'i.png',
        url        : 'u'
      })
    })

    test('omits the description key when it is absent', () => {
      const parsed = JSON.parse(articleLd({ headline: 'h', image: 'i.png', url: 'u' }))
      expect(parsed).not.toHaveProperty('description')
    })
  })

  describe('landingLd', () => {
    test('emits a WebSite and SoftwareApplication graph', () => {
      const { '@context': context, '@graph': graph } = JSON.parse(landingLd('https://prose.fyi/', '0.6.0', 'A typesetter.'))
      expect(context).toBe('https://schema.org')
      expect(graph).toHaveLength(2)
      expect(graph[0]).toMatchObject({ '@type': 'WebSite', description: 'A typesetter.', name: 'Prose', url: 'https://prose.fyi/' })
      expect(graph[1]).toMatchObject({ '@type': 'SoftwareApplication', name: 'Prose', softwareVersion: '0.6.0', url: 'https://prose.fyi/' })
    })

    test('drops the description when none is given', () => {
      const { '@graph': graph } = JSON.parse(landingLd('https://prose.fyi/', '0.6.0'))
      expect(graph[0]).not.toHaveProperty('description')
    })
  })
}
