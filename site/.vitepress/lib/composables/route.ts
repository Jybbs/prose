import { useData }                                                  from 'vitepress'
import { computed, inject, provide, type ComputedRef, type InjectionKey } from 'vue'

import { data as primitives, type DiscoveredPrimitive } from '../../data/primitives.data'
import { data as rules,      type RenderedRule }        from '../../data/rules.data'
import { FAMILY_META, type RuleFamily }                 from '../shared/registries'

const CURRENT_RULE_KEY: InjectionKey<ComputedRef<RenderedRule | null>> = Symbol('currentRule')

export function provideCurrentRule(): ComputedRef<RenderedRule | null> {
  const rule = useCurrentEntry('rules', rules.bySlug)
  provide(CURRENT_RULE_KEY, rule)
  return rule
}

export function useCurrentFamily(): ComputedRef<RuleFamily | null> {
  const { page } = useData()
  return computed(() => {
    const rel = page.value.relativePath
    if (!rel.startsWith('rules/')) return null
    const ruleSlug = rel.slice('rules/'.length).replace(/\.md$/, '')
    const ruleHit  = rules.bySlug[ruleSlug]
    if (ruleHit) return ruleHit.family
    const family = ruleSlug.split('/')[0]
    return family in FAMILY_META ? family as RuleFamily : null
  })
}

export const useCurrentPrimitive = (): ComputedRef<DiscoveredPrimitive | null> =>
  useCurrentEntry('primitives', primitives.bySlug)

export function useCurrentRule(): ComputedRef<RenderedRule | null> {
  return inject(CURRENT_RULE_KEY) ?? useCurrentEntry('rules', rules.bySlug)
}

function useCurrentEntry<T>(prefix: string, bySlug: Record<string, T>): ComputedRef<T | null> {
  const slug = useSlug(prefix)
  return computed(() => (slug.value && bySlug[slug.value]) ?? null)
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
