import { defineLoader } from 'vitepress'

import { inlineNodes, type InlineNode }                 from '../markdown/inline-nodes'
import { getRenderer, inlineNodeField }                 from '../markdown/renderer'
import { groupByMember }                                from '../shared/group-by-member'
import { architectureDir }                              from '../shared/paths'
import { PRIMITIVE_LAYERS, type PrimitiveLayer }        from '../shared/registries'
import { discoverPrimitives, type DiscoveredPrimitive } from './discovery'

interface PrimitiveEntry extends Omit<DiscoveredPrimitive, 'summary'> {
  linkNodes    : InlineNode[]
  summaryNodes : InlineNode[]
}

interface PrimitivesCompositionData {
  byLayer : Record<PrimitiveLayer, readonly PrimitiveEntry[]>
  bySlug  : Record<string, PrimitiveEntry>
  entries : readonly PrimitiveEntry[]
}

const dir = architectureDir(import.meta.url)

declare const data: PrimitivesCompositionData
export { data }

export default defineLoader({
  watch: [`${dir}/*.md`],
  async load(): Promise<PrimitivesCompositionData> {
    const md      = await getRenderer()
    const entries = inlineNodeField(md, discoverPrimitives(dir), 'summary')
      .map(entry => ({ ...entry, linkNodes: inlineNodes(md, `[[${entry.slug}]]`) }))
    const byLayer = groupByMember(entries, e => e.layer, PRIMITIVE_LAYERS)
    const bySlug  = Object.fromEntries(entries.map(e => [e.slug, e]))
    return { byLayer, bySlug, entries }
  }
})
