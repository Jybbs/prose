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

if (import.meta.vitest) {
  const { describe, expect, test } = import.meta.vitest

  const head = (...entries: Head): Head => entries
  const metaWith = (entries: Head, key: string, id: string) =>
    entries.find(entry => entry.tag === 'meta' && entry.attrs?.[key] === id)

  describe('canonicalOf', () => {
    test.each([
      { name: 'reads a canonical href',           entries: head({ tag: 'link', attrs: { rel: 'canonical', href: 'https://prose.fyi/x/' } }), expected: 'https://prose.fyi/x/' },
      { name: 'returns undefined with no link',    entries: head({ tag: 'meta', attrs: { name: 'x' } }),                                     expected: undefined            },
      { name: 'ignores a non-string href',         entries: head({ tag: 'link', attrs: { rel: 'canonical' } }),                              expected: undefined            }
    ])('$name', ({ entries, expected }) => {
      expect(canonicalOf(entries)).toBe(expected)
    })
  })

  describe('imageMeta', () => {
    test('pushes the og and twitter image tags with card dimensions', () => {
      const h = head()
      imageMeta(h, 'card.png', 'Alt text')
      expect(h.map(entry => [entry.attrs?.property ?? entry.attrs?.name, entry.attrs?.content])).toEqual([
        ['og:image',          'card.png'],
        ['og:image:width',    '1200'],
        ['og:image:height',   '630'],
        ['og:image:type',     'image/png'],
        ['og:image:alt',      'Alt text'],
        ['twitter:image',     'card.png'],
        ['twitter:image:alt', 'Alt text']
      ])
    })
  })

  describe('jsonLd', () => {
    test('appends a script tag carrying the payload', () => {
      const h = head()
      jsonLd(h, '{"x":1}')
      expect(h.at(-1)).toEqual({ attrs: { type: 'application/ld+json' }, content: '{"x":1}', tag: 'script' })
    })
  })

  describe('upsertMeta', () => {
    test('appends a fresh meta when none matches', () => {
      const h = head()
      upsertMeta(h, 'name', 'description', 'new')
      expect(metaWith(h, 'name', 'description')?.attrs?.content).toBe('new')
      expect(h).toHaveLength(1)
    })

    test('rewrites the content of an existing meta in place', () => {
      const h = head({ tag: 'meta', attrs: { content: 'old', name: 'description' } })
      upsertMeta(h, 'name', 'description', 'fresh')
      expect(h).toHaveLength(1)
      expect(metaWith(h, 'name', 'description')?.attrs?.content).toBe('fresh')
    })
  })
}
