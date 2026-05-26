import { useData }                                                         from 'vitepress'
import { computed, inject, provide, type ComputedRef, type InjectionKey }  from 'vue'

import { data as rules, type RenderedRule } from '../../data/rules.data'
import { FAMILY_META, type RuleFamily }     from '../shared/registries'

const CURRENT_RULE_KEY: InjectionKey<ComputedRef<RenderedRule | null>> = Symbol('currentRule')

function ruleSlug(rel: string): string | null {
  if (!rel.startsWith('rules/')) return null
  const slug = rel.slice('rules/'.length).replace(/\.md$/, '')
  return slug && slug !== 'index' ? slug : null
}

function buildCurrentRule(): ComputedRef<RenderedRule | null> {
  const { page } = useData()
  return computed(() => {
    const slug = ruleSlug(page.value.relativePath)
    return slug ? rules.bySlug[slug] ?? null : null
  })
}

export function provideCurrentRule(): ComputedRef<RenderedRule | null> {
  const entry = buildCurrentRule()
  provide(CURRENT_RULE_KEY, entry)
  return entry
}

export function useCurrentRule(): ComputedRef<RenderedRule | null> {
  return inject(CURRENT_RULE_KEY, null) ?? buildCurrentRule()
}

export function useCurrentFamily(): ComputedRef<RuleFamily | null> {
  const { page } = useData()
  return computed(() => {
    const slug = ruleSlug(page.value.relativePath)
    if (slug === null) return null
    const ruleHit = rules.bySlug[slug]
    if (ruleHit) return ruleHit.family
    const family = slug.split('/')[0]
    return family in FAMILY_META ? family as RuleFamily : null
  })
}
