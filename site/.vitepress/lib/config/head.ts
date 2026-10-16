import { inlineText }                  from '../markdown/inline-text'
import * as parts                      from '../og/render/parts'
import * as constants                  from '../shared/constants'
import { indexSectionOf, sectionOf }   from '../shared/registries'
import { canonicalUrl }                from './canonical-url'
import { articleLd, landingLd }        from './json-ld'
import { ogImageUrl, TOUCH_ICON_PATH } from './og-url'

import type { HeadConfig, PageData } from 'vitepress'

export const DESCRIPTION_FIELDS = ['description', 'caption', 'summary'] as const
export const TITLE_TEMPLATE     = `:title · ${constants.SITE_NAME}`

// Sets the description to the first `DESCRIPTION_FIELDS` entry the frontmatter
// holds as a string, flattened to plain text, titles a section's index page
// with its `SECTIONS` label, and titles the landing page with the site name and
// tagline in place of `TITLE_TEMPLATE`.
export function describePage(pageData: PageData): void {
  const source = DESCRIPTION_FIELDS.map(field => pageData.frontmatter[field])
    .find(value => typeof value === 'string')
  if (source !== undefined) pageData.description = inlineText(source)
  const section = indexSectionOf(pageData.relativePath)
  if (section !== undefined) pageData.title = section.label
  if (pageData.relativePath === 'index.md') {
    Object.assign(pageData, { title: constants.SITE_ALT, titleTemplate: false })
  }
}

export function pageHead(pageData: PageData, version: string): HeadConfig[] {
  const isLanding   = pageData.relativePath === 'index.md'
  const notFound    = pageData.isNotFound === true
  const landingCard = sectionOf(pageData.relativePath) === null
  const description = (notFound ? '' : pageData.description) || constants.SITE_TAGLINE
  const title       = pageData.frontmatter.name ?? pageData.title
  const ogImage     = ogImageUrl(pageData.relativePath)
  const ogTitle     = isLanding || notFound ? constants.SITE_NAME : title
  const ogAlt       = landingCard           ? constants.SITE_ALT  : `${title} card`
  const ogType      = landingCard           ? 'website'           : 'article'
  const head: HeadConfig[] = [
    ['link', {
      href  : `/${TOUCH_ICON_PATH}`,
      rel   : 'apple-touch-icon',
      sizes : `${parts.ICON_SIZE}x${parts.ICON_SIZE}`
    }],
    ['meta', { content: ogTitle,                   property: 'og:title'           }],
    ['meta', { content: description,               property: 'og:description'     }],
    ['meta', { content: ogType,                    property: 'og:type'            }],
    ['meta', { content: 'en_US',                   property: 'og:locale'          }],
    ['meta', { content: ogImage,                   property: 'og:image'           }],
    ['meta', { content: String(parts.CARD_WIDTH),  property: 'og:image:width'     }],
    ['meta', { content: String(parts.CARD_HEIGHT), property: 'og:image:height'    }],
    ['meta', { content: 'image/png',               property: 'og:image:type'      }],
    ['meta', { content: ogAlt,                     property: 'og:image:alt'       }],
    ['meta', { content: ogImage,                   name:     'twitter:image'      }],
    ['meta', { content: ogAlt,                     name:     'twitter:image:alt'  }]
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
