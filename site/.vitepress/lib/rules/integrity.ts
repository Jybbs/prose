import type { DiscoveredPrimitive }           from '../primitives/discovery'
import type { DiscoveredRule, RuleDiscovery } from './discovery'

// `consumedBy` names a primitive's consumers, which span rules, sibling
// primitives, and the crate's consumer surfaces, so the CLI and the
// WebAssembly bindings are legitimate consumers that own no primitive
// page of their own.
const SURFACE_CONSUMERS = new Set(['cli', 'wasm'])

export function assertCorpusIntegrity(
  discovery  : RuleDiscovery,
  primitives : readonly DiscoveredPrimitive[]
): void {
  const { rules, strayPages } = discovery
  if (strayPages.length > 0) {
    throw new Error(`Rule pages must live in a family directory, found stray: ${strayPages.join(', ')}`)
  }
  assertOneFamilyPerSlug(rules)
  const ruleSlugs = new Set(rules.map(r => r.slug))
  assertPrimitiveGraph(primitives, ruleSlugs)
  assertRelatedResolves(rules, ruleSlugs)
}

function assertOneFamilyPerSlug(rules: readonly DiscoveredRule[]): void {
  const known = new Set<string>()
  for (const { slug } of rules) {
    if (known.has(slug)) {
      throw new Error(`Rule "${slug}" has pages in more than one family directory`)
    }
    known.add(slug)
  }
}

function assertPrimitiveGraph(
  primitives : readonly DiscoveredPrimitive[],
  ruleSlugs  : ReadonlySet<string>
): void {
  const primitiveSlugs   = new Set<string>(primitives.map(p => p.slug))
  const consumerResolves = (name: string): boolean =>
    SURFACE_CONSUMERS.has(name) || primitiveSlugs.has(name) || ruleSlugs.has(name)
  for (const { consumedBy, consumes, slug } of primitives) {
    for (const dep of consumes) {
      if (!primitiveSlugs.has(dep)) {
        throw new Error(`Primitive "${slug}" consumes unknown primitive "${dep}"`)
      }
    }
    for (const consumer of consumedBy) {
      if (!consumerResolves(consumer)) {
        throw new Error(`Primitive "${slug}" lists unknown consumer "${consumer}"`)
      }
    }
  }
}

function assertRelatedResolves(
  rules     : readonly DiscoveredRule[],
  ruleSlugs : ReadonlySet<string>
): void {
  for (const { related, slug } of rules) {
    for (const ref of related) {
      if (!ruleSlugs.has(ref)) {
        throw new Error(`Rule "${slug}" lists invalid related slug "${ref}"`)
      }
    }
  }
}
