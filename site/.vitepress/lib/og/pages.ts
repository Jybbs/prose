import fs   from 'node:fs'
import path from 'node:path'

import matter from 'gray-matter'

import { type DiscoveredRule, discoverRuleSlugs }                       from '../rules/discovery'
import { parsePipeline }                                                from '../rules/pipeline-source'
import { FAMILY_META, PRIMITIVES, PUBLIC_PRIMITIVES, type PrimitiveSlug, type RuleCategory, type RuleFamily } from '../shared/registries'
import { toTitleCase }                                                  from '../shared/title-case'

const KINDS = ['integrations', 'primitives', 'reference', 'rules', 'usage'] as const
export type OgKind = typeof KINDS[number]

export interface OgPage {
  breadcrumb : readonly string[]
  caption   ?: string
  category  ?: RuleCategory
  family    ?: RuleFamily
  kind       : OgKind
  outputPath : string
  pipeline  ?: { position: number; total: number }
  primitive ?: { stability: 'internal' | 'public' }
  slug       : string
  title      : string
}

type PipelineEntry = { position: number; slug: string }

export function enumeratePages(srcDir: string, pages: readonly string[]): readonly OgPage[] {
  const rulesIndex   = new Map(discoverRuleSlugs(path.join(srcDir, 'rules')).map(r => [r.slug, r]))
  const pipeline     = parsePipeline(import.meta.url)
  const pipelinePos  = new Map(pipeline.map(r => [r.slug, r.position]))
  const out: OgPage[] = []
  for (const rel of pages) {
    if (rel === 'index.md') continue
    const kind = chapterKind(rel)
    if (kind === null) continue
    out.push(buildPage(rel, kind, rulesIndex, pipeline, pipelinePos, srcDir))
  }
  return out
}

function buildPage(
  rel         : string,
  kind        : OgKind,
  rulesIndex  : ReadonlyMap<string, DiscoveredRule>,
  pipeline    : readonly PipelineEntry[],
  pipelinePos : ReadonlyMap<string, number>,
  srcDir      : string
): OgPage {
  const slug       = pageSlug(rel)
  const outputPath = path.posix.join('og', rel.replace(/\.md$/, '.png'))
  if (rel.endsWith('/index.md')) {
    return { breadcrumb: [toTitleCase(kind, '-')], kind, outputPath, slug, title: indexTitle(rel, kind) }
  }
  if (kind === 'rules' && rulesIndex.has(slug)) {
    const rule     = rulesIndex.get(slug)!
    const position = pipelinePos.get(slug)
    return {
      breadcrumb : ['Rules', FAMILY_META[rule.family].label],
      caption    : rule.caption,
      category   : rule.category,
      family     : rule.family,
      kind,
      outputPath,
      pipeline   : position !== undefined ? { position, total: pipeline.length } : undefined,
      slug,
      title      : toTitleCase(slug, '-')
    }
  }
  if (kind === 'primitives') {
    const stability = PUBLIC_PRIMITIVES.includes(slug as PrimitiveSlug) ? 'public' : 'internal'
    return {
      breadcrumb : [toTitleCase(kind, '-')],
      kind,
      outputPath,
      primitive  : { stability },
      slug,
      title      : PRIMITIVES[slug as PrimitiveSlug] ?? toTitleCase(slug, '-')
    }
  }
  return {
    breadcrumb : [toTitleCase(kind, '-')],
    kind,
    outputPath,
    slug,
    title      : pageTitle(srcDir, rel)
  }
}

function chapterKind(rel: string): OgKind | null {
  const head = rel.split('/', 1)[0]
  return (KINDS as readonly string[]).includes(head) ? head as OgKind : null
}

function indexTitle(rel: string, kind: OgKind): string {
  if (rel === `${kind}/index.md`) return toTitleCase(kind, '-')
  const parts = rel.split('/')
  const tail  = parts[parts.length - 2]
  return toTitleCase(tail, '-')
}

function pageSlug(rel: string): string {
  const trimmed = rel.replace(/\.md$/, '').replace(/\/index$/, '')
  const tail    = trimmed.split('/').pop() ?? trimmed
  return tail || 'index'
}

function pageTitle(srcDir: string, rel: string): string {
  const body  = fs.readFileSync(path.join(srcDir, rel), 'utf8')
  const fm    = matter(body)
  const named = fm.data.name
  if (typeof named === 'string' && named.trim() !== '') return named.trim()
  const h1 = fm.content.match(/^#\s+(.+?)\s*$/m)?.[1]
  if (h1) return h1
  return toTitleCase(pageSlug(rel), '-')
}
