// Builds the element ids the per-rule facet catalogue anchors its rule groups
// and facets to, each facet sitting under its own rule's anchor so two rules
// declaring one key resolve to separate targets.

import { slugify } from '@mdit-vue/shared'

export const ALIGNMENT_SCOPE = 'alignment rules'
export const EVERY_RULE      = 'every rule'

// The scope each facet the shared sub-tables hoist reads under, those two
// reaching many rules where the rest reach one.
export const HOISTED_SCOPES: Record<string, string> = {
  'enabled'   : EVERY_RULE,
  'max-shift' : ALIGNMENT_SCOPE
}

export function facetAnchor(rule: string, key: string): string {
  return `${ruleAnchor(rule)}-${slugify(key)}`
}

// Returns the anchor a hoisted facet reads under, and `undefined` for a facet
// the catalogue lists beneath a rule of its own.
export function hoistedFacetAnchor(key: string): string | undefined {
  const scope = HOISTED_SCOPES[key]
  return scope === undefined ? undefined : facetAnchor(scope, key)
}

export function ruleAnchor(rule: string): string {
  return slugify(rule)
}
