import { computed, type ComputedRef } from 'vue'
import { useData } from 'vitepress'

import { data as rules, type DiscoveredRule } from '../data/rules.data'

function slugForPrefix(relativePath: string, prefix: string): string | null {
  const start = `${prefix}/`
  if (!relativePath.startsWith(start)) return null
  const slug = relativePath.slice(start.length).replace(/\.md$/, '')
  return slug && slug !== 'index' ? slug : null
}

function useSlug(prefix: string): ComputedRef<string | null> {
  const { page } = useData()
  return computed(() => slugForPrefix(page.value.relativePath, prefix))
}

export function useCurrentRule(): ComputedRef<DiscoveredRule | null> {
  const slug = useSlug('rules')
  return computed(() => rules.find(r => r.slug === slug.value) ?? null)
}

export function useIsRulePage(): ComputedRef<boolean> {
  const slug = useSlug('rules')
  return computed(() => slug.value !== null)
}

export const useCurrentPrimitive = (): ComputedRef<string | null> => useSlug('primitives')
