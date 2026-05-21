import { useData }                    from 'vitepress'
import { computed, type ComputedRef } from 'vue'

import { data as primitives, type DiscoveredPrimitive } from '../../data/primitives.data'
import { data as rules,      type RenderedRule }        from '../../data/rules.data'
import { DOMAIN_META, type RuleDomain }                 from './registries'

export function useCurrentPrimitive(): ComputedRef<DiscoveredPrimitive | null> {
  const slug = useSlug('primitives')
  return computed(() => (slug.value && primitives.bySlug[slug.value]) ?? null)
}

export function useCurrentRule(): ComputedRef<RenderedRule | null> {
  const slug = useSlug('rules')
  return computed(() => (slug.value && rules.bySlug[slug.value]) ?? null)
}

export function useCurrentDomain(): ComputedRef<RuleDomain | null> {
  const { page } = useData()
  return computed(() => {
    const rel = page.value.relativePath
    if (!rel.startsWith('rules/')) return null
    const ruleSlug = rel.slice('rules/'.length).replace(/\.md$/, '')
    const ruleHit  = rules.bySlug[ruleSlug]
    if (ruleHit) return ruleHit.domain
    const family = ruleSlug.split('/')[0]
    return family in DOMAIN_META ? family as RuleDomain : null
  })
}

function useSlug(prefix: string): ComputedRef<string | null> {
  const { page } = useData()
  return computed(() => {
    const start = `${prefix}/`
    const rel   = page.value.relativePath
    if (!rel.startsWith(start)) return null
    const slug = rel.slice(start.length).replace(/\.md$/, '')
    return slug && slug !== 'index' ? slug : null
  })
}
