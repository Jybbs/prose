import { defineLoader } from 'vitepress'

import { discoverPrimitiveIndex, discoverPrimitives } from '../lib/primitives/discovery'
import type { DiscoveredPrimitive }                   from '../lib/primitives/discovery'
import { primitivesDir }                              from '../lib/shared/paths'

interface PrimitivesData {
  bySlug : Record<string, DiscoveredPrimitive>
  list   : readonly DiscoveredPrimitive[]
}

const dir = primitivesDir(import.meta.url)

declare const data: PrimitivesData
export { data }

export default defineLoader({
  watch: [`${dir}/*.md`],
  load(): PrimitivesData {
    return {
      bySlug : Object.fromEntries(discoverPrimitiveIndex(dir)),
      list   : discoverPrimitives(dir)
    }
  }
})
