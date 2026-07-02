import { defineRouteMiddleware } from '@astrojs/starlight/route-data'
import { getCollection }         from 'astro:content'

import { isLandingId, ogImagePath }                   from '../og/url'
import { articleLd, landingLd }                       from './json-ld'
import { canonicalOf, imageMeta, jsonLd, upsertMeta } from './meta'

const LANDING_ALT = 'Prose, a Python typesetter for the reader.'

const [docs, [release]] = await Promise.all([getCollection('docs'), getCollection('release')])
const docsIds           = new Set(docs.map(entry => entry.id))

// Adds the per-route card image, Twitter image, description, and structured
// data onto the head Starlight built. A rule page's description falls back to
// its caption, and a route with no docs entry behind it, the 404 page, takes
// the landing card.
export const onRequest = defineRouteMiddleware(context => {
  const { site } = context
  if (site === undefined) return

  const { entry, head } = context.locals.starlightRoute
  const virtual     = !docsIds.has(entry.id)
  const landing     = isLandingId(entry.id)
  const description = entry.data.description ?? entry.data.caption
  const image       = new URL(ogImagePath(virtual ? 'index' : entry.id), site).href
  const alt         = landing || virtual ? LANDING_ALT : `${entry.data.title} card`
  imageMeta(head, image, alt)
  if (description !== undefined) {
    upsertMeta(head, 'name',     'description',    description)
    upsertMeta(head, 'property', 'og:description', description)
  }

  if (landing) {
    upsertMeta(head, 'property', 'og:type', 'website')
    jsonLd(head, landingLd(site.href, release.data.version, entry.data.description))
    return
  }
  const canonical = virtual ? undefined : canonicalOf(head)
  if (canonical !== undefined) {
    jsonLd(head, articleLd({
      description,
      headline : entry.data.title,
      image,
      url      : canonical
    }))
  }
})
