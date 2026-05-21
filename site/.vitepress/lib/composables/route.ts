import { useData }                                                  from 'vitepress'
import { computed, inject, provide, type ComputedRef, type InjectionKey } from 'vue'

import { data as primitives, type DiscoveredPrimitive } from '../../data/primitives.data'
import { data as rules,      type RenderedRule }        from '../../data/rules.data'
import { FAMILY_META, type RuleFamily }                 from '../shared/registries'

const CURRENT_RULE_KEY: InjectionKey<ComputedRef<RenderedRule | null>> = Symbol('currentRule')

export interface CurrentSlugs {
  primitive : ComputedRef<string | null>
  rule      : ComputedRef<string | null>
}

const SLUGS_KEY: InjectionKey<CurrentSlugs> = Symbol('currentSlugs')

export function provideCurrentSlugs(): CurrentSlugs {
  const { page } = useData()
  const slugFor  = (prefix: string) => computed(() => {
    const start = `${prefix}/`
    const rel   = page.value.relativePath
    if (!rel.startsWith(start)) return null
    const slug = rel.slice(start.length).replace(/\.md$/, '')
    return slug && slug !== 'index' ? slug : null
  })
  const slugs = { primitive: slugFor('primitives'), rule: slugFor('rules') }
  provide(SLUGS_KEY, slugs)
  return slugs
}

function useSlugs(): CurrentSlugs {
  const injected = inject(SLUGS_KEY, null)
  if (injected !== null) return injected
  const { page } = useData()
  const slugFor  = (prefix: string) => computed(() => {
    const start = `${prefix}/`
    const rel   = page.value.relativePath
    if (!rel.startsWith(start)) return null
    const slug = rel.slice(start.length).replace(/\.md$/, '')
    return slug && slug !== 'index' ? slug : null
  })
  return { primitive: slugFor('primitives'), rule: slugFor('rules') }
}

export function provideCurrentRule(): ComputedRef<RenderedRule | null> {
  const { rule } = useSlugs()
  const entry    = computed(() => (rule.value && rules.bySlug[rule.value]) ?? null)
  provide(CURRENT_RULE_KEY, entry)
  return entry
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

export function useCurrentPrimitive(): ComputedRef<DiscoveredPrimitive | null> {
  const { primitive } = useSlugs()
  return computed(() => (primitive.value && primitives.bySlug[primitive.value]) ?? null)
}

export function useCurrentRule(): ComputedRef<RenderedRule | null> {
  const injected = inject(CURRENT_RULE_KEY, null)
  if (injected !== null) return injected
  const { rule } = useSlugs()
  return computed(() => (rule.value && rules.bySlug[rule.value]) ?? null)
}
