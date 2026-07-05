import fs   from 'node:fs'
import path from 'node:path'

import matter from 'gray-matter'

import { markdownH1 }    from '../markdown/h1'
import { isContentPage } from '../shared/content-page'
import { memoizeByPath } from '../shared/memoize-by-path'
import * as registries   from '../shared/registries'
import { requireString } from '../shared/require-string'

export interface DiscoveredPrimitive {
  consumedBy : readonly string[]
  consumes   : readonly registries.PrimitiveSlug[]
  layer      : registries.PrimitiveLayer
  name       : string
  slug       : registries.PrimitiveSlug
  stability  : registries.PrimitiveStability
  summary    : string
  tagline    : string
}

function fieldMessage(slug: string, field: string): string {
  return `Primitive "${slug}" has invalid or missing ${field}`
}

function requireMember<T extends string>(
  value   : unknown,
  allowed : readonly T[],
  slug    : string,
  field   : string
): T {
  if (!allowed.includes(value as T)) {
    throw new Error(`${fieldMessage(slug, field)}: ${JSON.stringify(value)}`)
  }
  return value as T
}

function stringList(value: unknown, slug: string, field: string): string[] {
  if (!Array.isArray(value) || value.some(v => typeof v !== 'string')) {
    throw new Error(fieldMessage(slug, field))
  }
  return value as string[]
}

export const discoverPrimitives = memoizeByPath((primitivesDir): DiscoveredPrimitive[] => {
  const out: DiscoveredPrimitive[] = []
  for (const file of fs.readdirSync(primitivesDir).sort()) {
    if (!isContentPage(file)) continue
    const slug = path.basename(file, '.md') as registries.PrimitiveSlug
    const fm   = matter.read(path.join(primitivesDir, file))

    const stability = requireMember(fm.data.stability, registries.PRIMITIVE_STABILITIES, slug, 'stability')
    const layer     = requireMember(fm.data.layer, registries.PRIMITIVE_LAYERS, slug, 'layer')
    const summary   = requireString(fm.data.summary, fieldMessage(slug, 'summary'))
    const tagline   = requireString(fm.data.tagline, fieldMessage(slug, 'tagline'))

    const consumes   = stringList(fm.data.consumes, slug, 'consumes') as registries.PrimitiveSlug[]
    const consumedBy = stringList(fm.data.consumedBy, slug, 'consumedBy')

    const name = markdownH1(fm.content)
    if (!name) {
      throw new Error(`Primitive "${slug}" has no H1 heading`)
    }

    out.push({ consumedBy, consumes, layer, name, slug, stability, summary, tagline })
  }
  return out
})
