import { defineLoader } from 'vitepress'

import { getRenderer, renderInlineField }               from '../lib/markdown/renderer'
import { discoverPrimitives, type DiscoveredPrimitive } from '../lib/primitives/discovery'
import { primitivesDir }                                from '../lib/shared/paths'
import type { PrimitiveLayer }                          from '../lib/shared/registries'

type PrimitiveEntry = Omit<DiscoveredPrimitive, 'summary'> & { summaryHtml: string }

interface PrimitivesCompositionData {
  byLayer : Record<PrimitiveLayer, readonly PrimitiveEntry[]>
  entries : readonly PrimitiveEntry[]
}

const dir = primitivesDir(import.meta.url)

declare const data: PrimitivesCompositionData
export { data }

export default defineLoader({
  watch: [`${dir}/*.md`],
  async load(): Promise<PrimitivesCompositionData> {
    const md      = await getRenderer()
    const entries = renderInlineField(md, discoverPrimitives(dir), 'summary')
    type ByLayer  = Record<PrimitiveLayer, readonly PrimitiveEntry[]>
    const byLayer = Object.groupBy(entries, e => e.layer) as ByLayer
    return { byLayer, entries }
  }
})
