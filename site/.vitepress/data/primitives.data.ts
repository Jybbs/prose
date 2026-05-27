import { defineLoader } from 'vitepress'

import { discoverPrimitives, type DiscoveredPrimitive } from '../lib/primitives/discovery'
import { primitivesDir }                                from '../lib/shared/paths'

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
    const list   = discoverPrimitives(dir).slice().sort((a, b) => a.slug.localeCompare(b.slug))
    const bySlug = Object.fromEntries(list.map(p => [p.slug as string, p]))
    return { bySlug, list }
  }
})
