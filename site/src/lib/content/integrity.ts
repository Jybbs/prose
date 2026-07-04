import * as fc from 'fast-check'

import { isFamily }             from '../shared/registries'
import type { RuleFamily }      from '../shared/registries'
import { required }             from '../shared/required'
import { isIndex, slugOf }      from './discovery/page'
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

function assertStability(value: DocsFrontmatter['stability'], slug: string): void {
  required(value, `primitive "${slug}" is missing its stability`)
}

function assertWarmth(value: DocsFrontmatter['warmth'], family: RuleFamily): void {
  required(value, `family "${family}" index is missing its warmth`)
}

// Enforces the relationship invariants a per-record schema cannot reach over
// the loaded docs collection, throwing on the first violation to fail the
// build.
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
      const slug = slugOf(file)
      assertStability(data.stability, slug)
      primitives.push({
        consumedBy : data.consumedBy ?? [],
        consumes   : data.consumes ?? [],
        slug
      })
    }
  }

  if (strays.length > 0) {
    throw new Error(`rule pages must live in a family directory, found stray: ${strays.join(', ')}`)
  }
  assertOneFamilyPerSlug(rules)
  assertRelatedResolves(rules)
  assertPrimitiveGraph(primitives, rules)
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
// real node. The graph is not a strict inverse, because a primitive curates the
// consumers it lists rather than mirroring every edge.
function assertPrimitiveGraph(primitives: readonly Primitive[], rules: readonly Rule[]): void {
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

if (import.meta.vitest) {
  const { describe, expect, test } = import.meta.vitest

  const entry = (path: string, data: DocsFrontmatter = {}): CorpusEntry => ({ data, path })

  const VALID: CorpusEntry[] = [
    entry('rules/alignment/index.md',        { warmth: 'warm' }),
    entry('rules/alignment/align-equals.md', { caption: 'Aligns `=`', related: ['align-colons'] }),
    entry('rules/alignment/align-colons.md', { caption: 'Aligns `:`' }),
    entry('primitives/index.md'),
    entry('guide/getting-started.md'),
    entry('primitives/member.md', { consumedBy: ['cli', 'align-equals'], consumes: [],         stability: 'public' }),
    entry('primitives/band.md',   { consumedBy: ['cli', 'member'],       consumes: ['member'], stability: 'internal' })
  ]

  describe('assertCorpusIntegrity', () => {
    test.each([
      { name: 'a fully resolved corpus',              entries: VALID },
      { name: 'a slug repeated within one family',    entries: [
        entry('rules/alignment/dup.md', { caption: 'a' }),
        entry('rules/alignment/dup.md', { caption: 'b' })
      ] }
    ])('accepts $name', ({ entries }) => {
      expect(() => assertCorpusIntegrity(entries)).not.toThrow()
    })

    test.each([
      { name: 'a family index missing its warmth', message: /family "alignment" index is missing its warmth/, entries: [
        entry('rules/alignment/index.md')
      ] },
      { name: 'a rule page missing its caption', message: /rule "align-equals" is missing its caption/, entries: [
        entry('rules/alignment/align-equals.md')
      ] },
      { name: 'a rule page with a blank caption', message: /rule "align-equals" is missing its caption/, entries: [
        entry('rules/alignment/align-equals.md', { caption: '   ' })
      ] },
      { name: 'a primitive missing its stability', message: /primitive "member" is missing its stability/, entries: [
        entry('primitives/member.md')
      ] },
      { name: 'a rule page outside a family directory', message: /found stray: rules\/orphan\.md/, entries: [
        entry('rules/orphan.md', { caption: 'x' })
      ] },
      { name: 'a rule page under an unknown family', message: /found stray: rules\/bogus\/foo\.md/, entries: [
        entry('rules/bogus/foo.md', { caption: 'x' })
      ] },
      { name: 'a slug claimed by two families', message: /rule "dup" appears in both the alignment and layout families/, entries: [
        entry('rules/alignment/dup.md', { caption: 'a' }),
        entry('rules/layout/dup.md',    { caption: 'b' })
      ] },
      { name: 'a related reference that resolves to nothing', message: /rule "a" lists unknown related rule "ghost"/, entries: [
        entry('rules/alignment/a.md', { caption: 'x', related: ['ghost'] })
      ] },
      { name: 'a primitive consuming an unknown primitive', message: /primitive "band" consumes unknown primitive "ghost"/, entries: [
        entry('primitives/band.md', { consumes: ['ghost'], stability: 'internal' })
      ] },
      { name: 'a primitive listing an unknown consumer', message: /primitive "member" lists unknown consumer "ghost"/, entries: [
        entry('primitives/member.md', { consumedBy: ['ghost'], stability: 'public' })
      ] }
    ])('rejects $name', ({ entries, message }) => {
      expect(() => assertCorpusIntegrity(entries)).toThrow(message)
    })

    test('a single captioned rule page in a family never throws', () => {
      fc.assert(fc.property(fc.stringMatching(/^[a-z]{1,12}$/), slug => {
        expect(() => assertCorpusIntegrity([entry(`rules/alignment/${slug}.md`, { caption: 'x' })])).not.toThrow()
      }))
    })
  })
}
