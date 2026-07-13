import { defineLoader } from 'vitepress'

import { getRenderer, inlineNodeField }                 from '../markdown/renderer'
import type { InlineNode }                              from '../markdown/inline-nodes'
import { discoverPrimitives, type DiscoveredPrimitive } from './discovery'
import { primitivesDir }                                from '../shared/paths'
import type { PrimitiveLayer }                          from '../shared/registries'

type PrimitiveEntry = Omit<DiscoveredPrimitive, 'summary'> & { summaryNodes: InlineNode[] }

interface PrimitivesCompositionData {
  byLayer : Record<PrimitiveLayer, readonly PrimitiveEntry[]>
  bySlug  : Record<string, PrimitiveEntry>
  entries : readonly PrimitiveEntry[]
}

const dir = primitivesDir(import.meta.url)

declare const data: PrimitivesCompositionData
export { data }

export default defineLoader({
  watch: [`${dir}/*.md`],
  async load(): Promise<PrimitivesCompositionData> {
    const md      = await getRenderer()
    const entries = inlineNodeField(md, discoverPrimitives(dir), 'summary')
    type ByLayer  = Record<PrimitiveLayer, readonly PrimitiveEntry[]>
    const byLayer = Object.groupBy(entries, e => e.layer) as ByLayer
    const bySlug  = Object.fromEntries(entries.map(e => [e.slug as string, e]))
    return { byLayer, bySlug, entries }
  }
})
