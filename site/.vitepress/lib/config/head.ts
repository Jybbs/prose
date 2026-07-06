import { CARD_HEIGHT, CARD_WIDTH } from '../og/render/parts'
import { SITE_ALT, SITE_TAGLINE }  from '../shared/constants'
import { canonicalUrl }            from './canonical-url'
import { articleLd, landingLd }    from './json-ld'
import { ogImageUrl }              from './og-url'

import type { HeadConfig, PageData } from 'vitepress'

export function pageHead(pageData: PageData, version: string): HeadConfig[] {
  const isLanding   = pageData.relativePath === 'index.md'
  const notFound    = pageData.isNotFound === true
  const landingCard = isLanding || notFound
  const description = notFound ? SITE_TAGLINE : pageData.description || SITE_TAGLINE
  const title       = pageData.frontmatter.name ?? pageData.title
  const ogImage     = ogImageUrl(landingCard ? 'index.md' : pageData.relativePath)
  const ogTitle     = isLanding   ? 'Prose'   : `${title} · Prose`
  const ogAlt       = landingCard ? SITE_ALT  : `${title} card`
  const ogType      = landingCard ? 'website' : 'article'
  const head: HeadConfig[] = [
    ['meta', { content: ogTitle,             property: 'og:title'           }],
    ['meta', { content: description,         property: 'og:description'     }],
    ['meta', { content: ogType,              property: 'og:type'            }],
    ['meta', { content: 'en_US',             property: 'og:locale'          }],
    ['meta', { content: ogImage,             property: 'og:image'           }],
    ['meta', { content: String(CARD_WIDTH),  property: 'og:image:width'     }],
    ['meta', { content: String(CARD_HEIGHT), property: 'og:image:height'    }],
    ['meta', { content: 'image/png',         property: 'og:image:type'      }],
    ['meta', { content: ogAlt,               property: 'og:image:alt'       }],
    ['meta', { content: ogImage,             name:     'twitter:image'      }],
    ['meta', { content: ogAlt,               name:     'twitter:image:alt'  }]
  ]
  if (notFound) return head
  const ogUrl  = canonicalUrl(pageData.relativePath)
  const jsonLd = isLanding
    ? landingLd(ogUrl, version, description)
    : articleLd({ description, headline: title, image: ogImage, url: ogUrl })
  head.push(
    ['meta',   { content: ogUrl, property: 'og:url' }],
    ['script', { type: 'application/ld+json' }, jsonLd]
  )
  return head
}
