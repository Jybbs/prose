import { isFamily }             from '../shared/registries'
import type { RuleFamily }      from '../shared/registries'
import { isIndex, slugOf }      from './page'
import type { DocsFrontmatter } from './schemas'

// `consumedBy` names a primitive's consumers, which span rules, sibling
// primitives, and the CLI, so the CLI is a legitimate consumer that owns no
// primitive page of its own.
const CLI_CONSUMER = 'cli'

// One docs entry plus its path relative to `src/content/docs`, which carries
// the section and family the cross-record checks read off the directory tree.
export interface CorpusEntry {
  data : DocsFrontmatter
  path : string
}

interface Rule {
  family  : RuleFamily
  related : readonly string[]
  slug    : string
}

interface Primitive {
  consumedBy : readonly string[]
  consumes   : readonly string[]
  slug       : string
}

function assertCaption(value: string | undefined, slug: string): void {
  if (typeof value !== 'string' || value.trim() === '') {
    throw new Error(`rule "${slug}" is missing its caption`)
  }
}

function assertWarmth(value: DocsFrontmatter['warmth'], family: RuleFamily): void {
  if (value === undefined) throw new Error(`family "${family}" index is missing its warmth`)
}

// Enforces the relationship invariants a per-record schema cannot reach over
// the loaded docs collection, throwing on the first violation to fail the
// build. Covers stray-page rejection and family-directory legitimacy,
// family-index warmth, one family per rule slug, `related` resolution, and
// the primitive consumes-and-consumed-by graph.
export function assertCorpusIntegrity(entries: Iterable<CorpusEntry>): void {
  const primitives : Primitive[] = []
  const rules      : Rule[]      = []
  const strays     : string[]    = []

  for (const { data, path } of entries) {
    const parts = path.split('/')
    const file  = parts.at(-1) ?? ''
    if (parts[0] === 'rules' && parts.length === 3 && isFamily(parts[1]) && isIndex(file)) {
      assertWarmth(data.warmth, parts[1])
      continue
    }
    if (isIndex(file)) continue

    if (parts[0] === 'rules') {
      const family = parts[1]
      if (parts.length !== 3 || !isFamily(family)) {
        strays.push(path)
        continue
      }
      const slug = slugOf(file)
      assertCaption(data.caption, slug)
      rules.push({
        family,
        related: data.related ?? [],
        slug
      })
    } else if (parts[0] === 'primitives' && parts.length === 2) {
      primitives.push({
        consumedBy : data.consumedBy ?? [],
        consumes   : data.consumes ?? [],
        slug       : slugOf(file)
      })
    }
  }

  if (strays.length > 0) {
    throw new Error(`rule pages must live in a family directory, found stray: ${strays.join(', ')}`)
  }
  assertOneFamilyPerSlug(rules)
  assertRelatedResolves(rules)
  assertPrimitiveGraph(rules, primitives)
}

function assertOneFamilyPerSlug(rules: readonly Rule[]): void {
  const placed = new Map<string, RuleFamily>()
  for (const { family, slug } of rules) {
    const prior = placed.get(slug)
    if (prior !== undefined && prior !== family) {
      throw new Error(`rule "${slug}" appears in both the ${prior} and ${family} families`)
    }
    placed.set(slug, family)
  }
}

function assertRelatedResolves(rules: readonly Rule[]): void {
  const slugs = new Set(rules.map(r => r.slug))
  for (const { related, slug } of rules) {
    for (const ref of related) {
      if (!slugs.has(ref)) throw new Error(`rule "${slug}" lists unknown related rule "${ref}"`)
    }
  }
}

// Validates that every edge of the consumes-and-consumed-by graph resolves to a
// real node. A `consumes` edge names another primitive, whereas a `consumedBy`
// edge names a consumer the primitive serves, which spans rules, sibling
// primitives, and the CLI. The graph is not a strict inverse, because a
// primitive curates the consumers it lists rather than mirroring every edge.
function assertPrimitiveGraph(rules: readonly Rule[], primitives: readonly Primitive[]): void {
  const primitiveSlugs = new Set(primitives.map(p => p.slug))
  const ruleSlugs      = new Set(rules.map(r => r.slug))
  const consumerOk     = (name: string): boolean =>
    name === CLI_CONSUMER || primitiveSlugs.has(name) || ruleSlugs.has(name)

  for (const { consumedBy, consumes, slug } of primitives) {
    for (const dep of consumes) {
      if (!primitiveSlugs.has(dep)) {
        throw new Error(`primitive "${slug}" consumes unknown primitive "${dep}"`)
      }
    }
    for (const consumer of consumedBy) {
      if (!consumerOk(consumer)) {
        throw new Error(`primitive "${slug}" lists unknown consumer "${consumer}"`)
      }
    }
  }
}
