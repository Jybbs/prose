import type { StarlightRouteData } from '@astrojs/starlight/route-data'

import { CARD_HEIGHT, CARD_WIDTH } from '../og/parts'

type Head      = StarlightRouteData['head']
type HeadEntry = Head[number]

// The canonical URL Starlight already computed for the page, read back off the
// head so the structured data cannot drift from the emitted link.
export function canonicalOf(head: Head): string | undefined {
  const link = head.find(entry => entry.tag === 'link' && entry.attrs?.rel === 'canonical')
  return typeof link?.attrs?.href === 'string' ? link.attrs.href : undefined
}

export function imageMeta(head: Head, image: string, alt: string): void {
  head.push(
    meta('property', 'og:image',         image),
    meta('property', 'og:image:width',   String(CARD_WIDTH)),
    meta('property', 'og:image:height',  String(CARD_HEIGHT)),
    meta('property', 'og:image:type',    'image/png'),
    meta('property', 'og:image:alt',     alt),
    meta('name',     'twitter:image',     image),
    meta('name',     'twitter:image:alt', alt)
  )
}

export function jsonLd(head: Head, content: string): void {
  head.push({ attrs: { type: 'application/ld+json' }, content, tag: 'script' })
}

export function upsertMeta(
  head    : Head,
  key     : 'name' | 'property',
  id      : string,
  content : string
): void {
  const existing = head.find(entry => entry.tag === 'meta' && entry.attrs?.[key] === id)
  if (existing?.attrs !== undefined) existing.attrs.content = content
  else head.push(meta(key, id, content))
}

function meta(key: 'name' | 'property', id: string, content: string): HeadEntry {
  return { attrs: { content, [key]: id }, tag: 'meta' }
}
