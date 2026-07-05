const CONTEXT = 'https://schema.org'

interface ArticleFacts {
  description : string
  headline    : string
  image       : string
  url         : string
}

export function articleLd(facts: ArticleFacts): string {
  return JSON.stringify({ '@context': CONTEXT, '@type': 'TechArticle', ...facts })
}

export function landingLd(url: string, version: string, description: string): string {
  return JSON.stringify({
    '@context' : CONTEXT,
    '@graph'   : [
      { '@type': 'WebSite', description, name: 'Prose', url },
      {
        '@type'             : 'SoftwareApplication',
        applicationCategory : 'DeveloperApplication',
        license             : 'https://opensource.org/license/mit',
        name                : 'Prose',
        operatingSystem     : 'Linux, macOS, Windows',
        softwareVersion     : version,
        url
      }
    ]
  })
}
