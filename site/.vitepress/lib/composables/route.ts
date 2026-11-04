import { useData } from 'vitepress'
import { computed, inject, provide, type ComputedRef, type InjectionKey } from 'vue'

import { data as rules, type RenderedRule } from '../rules/rules.data'
import { stripSuffix }                      from '../shared/strip-suffix'

const CURRENT_RULE_KEY: InjectionKey<ComputedRef<RenderedRule | null>> = Symbol('currentRule')

function routeSegments(rel: string): readonly string[] {
  if (!rel.startsWith('rules/')) return []
  return stripSuffix(rel.slice('rules/'.length), '.md').split('/')
}

function buildCurrentRule(): ComputedRef<RenderedRule | null> {
  const { page } = useData()
  return computed(() => {
    const slug = routeSegments(page.value.relativePath).at(-1)
    return slug && slug !== 'index' ? rules.bySlug[slug] ?? null : null
  })
}

export function provideCurrentRule(): void {
  provide(CURRENT_RULE_KEY, buildCurrentRule())
}

export function useCurrentRule(): ComputedRef<RenderedRule | null> {
  return inject(CURRENT_RULE_KEY, null) ?? buildCurrentRule()
}
