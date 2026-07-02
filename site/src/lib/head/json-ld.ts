import type { Graph, TechArticle, WithContext } from 'schema-dts'

const CONTEXT = 'https://schema.org'

interface ArticleFacts {
  description ?: string
  headline     : string
  image        : string
  url          : string
}

// One `TechArticle` node per docs page, carrying the headline, description,
// canonical URL, and the page's card as its image.
export function articleLd(facts: ArticleFacts): string {
  const article: WithContext<TechArticle> = { '@context': CONTEXT, '@type': 'TechArticle', ...facts }
  return JSON.stringify(article)
}

// The landing graph: the `WebSite` node plus the `SoftwareApplication` node
// describing the formatter itself.
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
