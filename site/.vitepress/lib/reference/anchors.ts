import { slugify } from '@mdit-vue/shared'

// Builds the element ids the per-rule facet catalogue anchors its rule groups
// and facets to, each facet sitting under its own rule's anchor so two rules
// declaring one key resolve to separate targets.
export function facetAnchor(rule: string, key: string): string {
  return `${ruleAnchor(rule)}-${slugify(key)}`
}

export function ruleAnchor(rule: string): string {
  return slugify(rule)
}
