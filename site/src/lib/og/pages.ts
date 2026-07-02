import { getCollection }        from 'astro:content'
import type { CollectionEntry } from 'astro:content'

import { isFamily }        from '../shared/registries'
import type { RuleFamily } from '../shared/registries'
import { resolveColor }    from '../tokens/resolve'
import { isLandingId }     from './url'

type DocsEntry = CollectionEntry<'docs'>
type Warmth    = NonNullable<DocsEntry['data']['warmth']>

export interface OgPage {
  accent    ?: string
  breadcrumb : readonly string[]
  caption   ?: string
  family    ?: RuleFamily
  kind       : string
  pipeline  ?: { position: number, total: number }
  stability ?: string
  title      : string
  warmth    ?: Warmth
}

export interface OgCard {
  id   : string
  page : OgPage | 'landing'
}

// Enumerates every card the build renders, the landing card plus one card per
// docs page, enriched from the docs and pipeline collections.
export async function enumerateCards(): Promise<OgCard[]> {
  const [docs, pipeline] = await Promise.all([getCollection('docs'), getCollection('pipeline')])
  const positions = new Map(pipeline.map(entry => [entry.data.slug, entry.data.position]))
  const warmths   = familyWarmths(docs)
  return [
    { id: 'index', page: 'landing' },
    ...docs
      .filter(entry => !isLandingId(entry.id))
      .map(entry => ({ id: entry.id, page: pageFor(entry, positions, pipeline.length, warmths) }))
  ]
}

function accentFor(kind: string, family?: RuleFamily): string | undefined {
  return resolveColor(family !== undefined ? `family-${family}` : `section-${kind}`) || undefined
}

function familyWarmths(docs: readonly DocsEntry[]): Map<RuleFamily, Warmth> {
  const out = new Map<RuleFamily, Warmth>()
  for (const entry of docs) {
    const [head, family, ...rest] = entry.id.split('/')
    if (head !== 'rules' || rest.length > 0 || family === undefined || !isFamily(family)) continue
    if (entry.data.warmth !== undefined) out.set(family, entry.data.warmth)
  }
  return out
}

function pageFor(
  entry     : DocsEntry,
  positions : ReadonlyMap<string, number>,
  total     : number,
  warmths   : ReadonlyMap<RuleFamily, Warmth>
): OgPage {
  const parts = entry.id.split('/')
  const kind  = parts[0]
  const base  = { accent: accentFor(kind), breadcrumb: [kind], kind, title: entry.data.title }
  if (kind === 'rules' && parts.length === 3 && isFamily(parts[1])) {
    const family   = parts[1]
    const position = positions.get(parts[2])
    return {
      ...base,
      accent     : accentFor(kind, family),
      breadcrumb : [kind, family],
      caption    : entry.data.caption,
      family,
      pipeline   : position !== undefined ? { position, total } : undefined,
      title      : titleCase(parts[2]),
      warmth     : warmths.get(family)
    }
  }
  if (kind === 'rules' && parts.length === 2 && isFamily(parts[1])) {
    return { ...base, accent: accentFor(kind, parts[1]) }
  }
  if (kind === 'primitives' && parts.length === 2) {
    return { ...base, stability: entry.data.stability ?? 'internal' }
  }
  return base
}

function titleCase(slug: string): string {
  return slug.split('-').map(word => word.charAt(0).toUpperCase() + word.slice(1)).join(' ')
}
