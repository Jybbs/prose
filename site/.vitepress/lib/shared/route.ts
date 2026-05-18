import { useData }                    from 'vitepress'
import { computed, type ComputedRef } from 'vue'

import { data as primitives, type DiscoveredPrimitive } from '../../data/primitives.data'
import { data as rules,      type DiscoveredRule }      from '../../data/rules.data'

export function slugForPrefix(relativePath: string, prefix: string): string | null {
  const start = `${prefix}/`
  if (!relativePath.startsWith(start)) return null
  const slug = relativePath.slice(start.length).replace(/\.md$/, '')
  return slug && slug !== 'index' ? slug : null
}

export function useCurrentPrimitive(): ComputedRef<DiscoveredPrimitive | null> {
  const slug = useSlug('primitives')
  return computed(() => (slug.value && primitives.bySlug[slug.value]) ?? null)
}

export function useCurrentRule(): ComputedRef<DiscoveredRule | null> {
  const slug = useSlug('rules')
  return computed(() => (slug.value && rules.bySlug[slug.value]) ?? null)
}

function useSlug(prefix: string): ComputedRef<string | null> {
  const { page } = useData()
  return computed(() => slugForPrefix(page.value.relativePath, prefix))
}
