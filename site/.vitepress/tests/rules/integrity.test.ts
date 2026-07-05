import type { DiscoveredPrimitive } from '../../lib/primitives/discovery'
import type { DiscoveredRule }      from '../../lib/rules/discovery'
import { assertCorpusIntegrity }    from '../../lib/rules/integrity'

const rule = (slug: string, related: readonly string[] = []): DiscoveredRule => ({
  caption  : 'caption',
  category : 'auto-fix',
  family   : 'alignment',
  href     : `/rules/alignment/${slug}`,
  related,
  slug
})

type PrimitiveOverrides = Partial<Omit<DiscoveredPrimitive, 'consumes'>> & {
  consumes?: readonly string[]
}

const primitive = (
  slug      : string,
  overrides : PrimitiveOverrides = {}
): DiscoveredPrimitive => ({
  consumedBy : [],
  consumes   : [],
  layer      : 'base',
  name       : slug,
  slug       : slug as DiscoveredPrimitive['slug'],
  stability  : 'public',
  summary    : 'summary',
  tagline    : 'tagline',
  ...overrides
} as DiscoveredPrimitive)

const corpus = (
  rules      : DiscoveredRule[],
  primitives : DiscoveredPrimitive[]      = [],
  strayPages : string[]                   = []
) => () => assertCorpusIntegrity({ rules, strayPages }, primitives)

describe('assertCorpusIntegrity', () => {
  it('accepts a fully resolved corpus', () => {
    expect(corpus(
      [rule('align-equals', ['align-colons']), rule('align-colons')],
      [
        primitive('member', { consumedBy: ['cli', 'align-equals', 'band'] }),
        primitive('band',   { consumes: ['member'] })
      ]
    )).not.toThrow()
  })

  it.each([
    ['a stray page outside a family directory', /must live in a family directory/,
      corpus([], [], ['orphan.md'])],
    ['a slug with pages in two families',       /more than one family directory/,
      corpus([rule('dup'), { ...rule('dup'), family: 'layout' }])],
    ['a dangling related slug',                 /lists invalid related slug "ghost"/,
      corpus([rule('a', ['ghost'])])],
    ['an unknown consumed primitive',           /consumes unknown primitive "ghost"/,
      corpus([], [primitive('band', { consumes: ['ghost'] })])],
    ['an unknown consumedBy consumer',          /lists unknown consumer "ghost"/,
      corpus([], [primitive('member', { consumedBy: ['ghost'] })])]
  ])('rejects %s', (_name, message, run) => {
    expect(run).toThrow(message)
  })
})
