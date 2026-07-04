import { getCollection }        from 'astro:content'
import type { CollectionEntry } from 'astro:content'

import { familyIndex, familyPage, sectionLeaf } from '../content/discovery/sections'
import { isFamily }                             from '../shared/registries'
import type { PrimitiveStability, RuleFamily }  from '../shared/registries'
import { titleCase }                            from '../shared/title-case'
import { resolveColor }                         from '../tokens/resolve'
import type { TokenName }                       from '../tokens/resolve'
import { isLandingId }                          from './url'

type DocsEntry = CollectionEntry<'docs'>
type Warmth    = NonNullable<DocsEntry['data']['warmth']>

export interface OgPage {
  accent    ?: string
  breadcrumb : readonly string[]
  caption   ?: string
  family    ?: RuleFamily
  kind       : string
  pipeline  ?: { position: number, total: number }
  stability ?: PrimitiveStability
  title      : string
  warmth    ?: Warmth
}

export interface OgCard {
  id   : string
  page : OgPage | 'landing'
}

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
  const token = family !== undefined ? `family-${family}` : `section-${kind}`
  return resolveColor(token as TokenName) || undefined
}

function familyWarmths(docs: readonly DocsEntry[]): Map<RuleFamily, Warmth> {
  const out = new Map<RuleFamily, Warmth>()
  for (const entry of docs) {
    const family = familyIndex(entry.id)
    if (family === undefined || !isFamily(family) || entry.data.warmth === undefined) continue
    out.set(family, entry.data.warmth)
  }
  return out
}

function pageFor(
  entry     : DocsEntry,
  positions : ReadonlyMap<string, number>,
  total     : number,
  warmths   : ReadonlyMap<RuleFamily, Warmth>
): OgPage {
  const kind = entry.id.split('/')[0]
  const base = { accent: accentFor(kind), breadcrumb: [kind], kind, title: entry.data.title }

  const rule = familyPage(entry.id)
  if (rule !== undefined && isFamily(rule.family)) {
    const { family, slug } = rule
    const position         = positions.get(slug)
    return {
      ...base,
      accent     : accentFor(kind, family),
      breadcrumb : [kind, family],
      caption    : entry.data.caption,
      family,
      pipeline   : position !== undefined ? { position, total } : undefined,
      title      : titleCase(slug),
      warmth     : warmths.get(family)
    }
  }

  const family = familyIndex(entry.id)
  if (family !== undefined && isFamily(family)) {
    return { ...base, accent: accentFor(kind, family) }
  }

  if (sectionLeaf(entry.id, 'primitives') !== undefined) {
    return { ...base, stability: entry.data.stability ?? 'internal' }
  }
  return base
}
