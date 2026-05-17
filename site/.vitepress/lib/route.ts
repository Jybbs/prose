import { computed, type ComputedRef } from 'vue'
import { useRoute } from 'vitepress'

import { data as rules } from '../data/rules.data'
import type { DiscoveredRule } from './rules'

const RULE_PATH_PATTERN = /^\/rules\/([a-z0-9-]+)(?:\.html)?$/

export function useCurrentRule(): ComputedRef<DiscoveredRule | null> {
  const route = useRoute()
  return computed(() => {
    const match = route.path.match(RULE_PATH_PATTERN)
    if (!match) return null
    return rules.find(r => r.slug === match[1]) ?? null
  })
}

export function useIsRulePage(): ComputedRef<boolean> {
  const route = useRoute()
  return computed(() => RULE_PATH_PATTERN.test(route.path))
}
