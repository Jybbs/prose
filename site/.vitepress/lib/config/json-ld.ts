import { SITE_NAME } from '../shared/constants'

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
      { '@type': 'WebSite', description, name: SITE_NAME, url },
      {
        '@type'             : 'SoftwareApplication',
        applicationCategory : 'DeveloperApplication',
        license             : 'https://opensource.org/license/mit',
        name                : SITE_NAME,
        operatingSystem     : 'Linux, macOS, Windows',
        softwareVersion     : version,
        url
      }
    ]
  })
}
