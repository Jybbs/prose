import { defineLoader } from 'vitepress'

import { discoverPrimitiveIndex, discoverPrimitives } from './discovery'
import type { DiscoveredPrimitive }                   from './discovery'
import { primitivesDir }                              from '../shared/paths'

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
