import path from 'node:path'

import matter from 'gray-matter'

import { ogImagePath }                                     from '../config/og-url'
import { markdownH1 }                                      from '../markdown/h1'
import { type DiscoveredPrimitive, discoverPrimitives }    from '../primitives/discovery'
import { type DiscoveredRule, discoverRuleSlugs }          from '../rules/discovery'
import { parsePipeline }                                   from '../rules/pipeline-source'
import { resolveToken }                                    from '../shared/css-token'
import { FAMILY_META, type RuleCategory, type RuleFamily } from '../shared/registries'
import { toTitleCase }                                     from '../shared/title-case'

const KINDS = ['integrations', 'primitives', 'reference', 'rules', 'usage'] as const
type OgKind = typeof KINDS[number]

export interface OgPage {
  accent    ?: string
  breadcrumb : readonly string[]
  caption   ?: string
  category  ?: RuleCategory
  family    ?: RuleFamily
  kind       : OgKind
  outputPath : string
  pipeline  ?: { position: number; total: number }
  primitive ?: { stability: 'internal' | 'public' }
  title      : string
}

export function enumeratePages(srcDir: string, pages: readonly string[]): readonly OgPage[] {
  const rulesIndex      = new Map(discoverRuleSlugs(path.join(srcDir, 'rules')).map(r => [r.slug, r]))
  const primitivesIndex = new Map(
    discoverPrimitives(path.join(srcDir, 'primitives')).map(p => [p.slug as string, p])
  )
  const pipeline        = parsePipeline(import.meta.url)
  const pipelinePos     = new Map(pipeline.map(r => [r.slug, r.position]))
  const out: OgPage[]   = []
  for (const rel of pages) {
    if (rel === 'index.md') continue
    const kind = chapterKind(rel)
    if (kind === null) continue
    out.push(buildPage(rel, kind, rulesIndex, primitivesIndex, pipeline.length, pipelinePos, srcDir))
  }
  return out
}

function accentFor(kind: OgKind, family?: RuleFamily): string | undefined {
  const token = family !== undefined ? `prose-c-family-${family}` : `prose-c-section-${kind}`
  return resolveToken(token) || undefined
}

function buildPage(
  rel             : string,
  kind            : OgKind,
  rulesIndex      : ReadonlyMap<string, DiscoveredRule>,
  primitivesIndex : ReadonlyMap<string, DiscoveredPrimitive>,
  pipelineTotal   : number,
  pipelinePos     : ReadonlyMap<string, number>,
  srcDir          : string
): OgPage {
  const slug       = pageSlug(rel)
  const outputPath = ogImagePath(rel)
  if (rel.endsWith('/index.md')) {
    return {
      accent     : accentFor(kind, indexFamily(rel)),
      breadcrumb : [toTitleCase(kind, '-')],
      kind,
      outputPath,
      title      : indexTitle(rel, kind)
    }
  }
  if (kind === 'rules' && rulesIndex.has(slug)) {
    const rule     = rulesIndex.get(slug)!
    const position = pipelinePos.get(slug)
    return {
      accent     : accentFor(kind, rule.family),
      breadcrumb : ['Rules', FAMILY_META[rule.family].label],
      caption    : rule.caption,
      category   : rule.category,
      family     : rule.family,
      kind,
      outputPath,
      pipeline   : position !== undefined ? { position, total: pipelineTotal } : undefined,
      title      : toTitleCase(slug, '-')
    }
  }
  if (kind === 'primitives') {
    const primitive = primitivesIndex.get(slug)
    return {
      accent     : accentFor(kind),
      breadcrumb : [toTitleCase(kind, '-')],
      kind,
      outputPath,
      primitive  : { stability: primitive?.stability ?? 'internal' },
      title      : primitive?.name ?? toTitleCase(slug, '-')
    }
  }
  return {
    accent     : accentFor(kind),
    breadcrumb : [toTitleCase(kind, '-')],
    kind,
    outputPath,
    title      : pageTitle(srcDir, rel)
  }
}

function chapterKind(rel: string): OgKind | null {
  const head = rel.split('/', 1)[0]
  return (KINDS as readonly string[]).includes(head) ? head as OgKind : null
}

function indexFamily(rel: string): RuleFamily | undefined {
  const dir = rel.split('/').at(-2)
  return dir !== undefined && dir in FAMILY_META ? dir as RuleFamily : undefined
}

function indexTitle(rel: string, kind: OgKind): string {
  if (rel === `${kind}/index.md`) return toTitleCase(kind, '-')
  const parts = rel.split('/')
  const tail  = parts.at(-2)!
  return toTitleCase(tail, '-')
}

function pageSlug(rel: string): string {
  const stem = path.basename(rel, '.md')
  return stem === 'index' ? path.basename(path.dirname(rel)) || 'index' : stem
}

function pageTitle(srcDir: string, rel: string): string {
  const fm    = matter.read(path.join(srcDir, rel))
  const named = typeof fm.data.name === 'string' ? fm.data.name.trim() : ''
  if (named) return named
  const h1 = markdownH1(fm.content)
  if (h1) return h1
  return toTitleCase(pageSlug(rel), '-')
}
