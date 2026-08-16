import { defineLoader } from 'vitepress'

import { getRenderer, inlineNodeField }           from '../markdown/renderer'
import { inlineNodes, type InlineNode }           from '../markdown/inline-nodes'
import { discoverPrimitives }                     from './discovery'
import { primitivesDir }                          from '../shared/paths'
import type { PrimitiveSlug, PrimitiveStability } from '../shared/registries'

export interface PrimitiveSurfaceRow {
  linkNodes    : InlineNode[]
  slug         : PrimitiveSlug
  stability    : PrimitiveStability
  summaryNodes : InlineNode[]
}

const dir = primitivesDir(import.meta.url)

declare const data: readonly PrimitiveSurfaceRow[]
export { data }

export default defineLoader({
  watch: [`${dir}/*.md`],
  async load(): Promise<readonly PrimitiveSurfaceRow[]> {
    const md = await getRenderer()
    return inlineNodeField(md, discoverPrimitives(dir), 'summary')
      .map(({ slug, stability, summaryNodes }) => ({
        linkNodes    : inlineNodes(md, `[[${slug}]]`),
        slug         : slug,
        stability    : stability,
        summaryNodes : summaryNodes
      }))
  }
})
