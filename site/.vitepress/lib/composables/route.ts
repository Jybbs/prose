import { useData }                                                         from 'vitepress'
import { computed, inject, provide, type ComputedRef, type InjectionKey }  from 'vue'

import { data as rules, type RenderedRule } from '../../data/rules.data'
import { FAMILY_META, type RuleFamily }     from '../shared/registries'

const CURRENT_RULE_KEY: InjectionKey<ComputedRef<RenderedRule | null>> = Symbol('currentRule')

function routeSegments(rel: string): readonly string[] {
  if (!rel.startsWith('rules/')) return []
  return rel.slice('rules/'.length).replace(/\.md$/, '').split('/')
}

function buildCurrentRule(): ComputedRef<RenderedRule | null> {
  const { page } = useData()
  return computed(() => {
    const slug = routeSegments(page.value.relativePath).at(-1)
    return slug && slug !== 'index' ? rules.bySlug[slug] ?? null : null
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
    const family = routeSegments(page.value.relativePath)[0]
    return family && family in FAMILY_META ? family as RuleFamily : null
  })
}
