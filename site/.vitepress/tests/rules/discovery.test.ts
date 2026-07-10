import * as discovery from '../../lib/rules/discovery'
import * as support   from '../support'

describe('discoverRules', () => {
  const fixture = (name: string): string => support.fixtureDir(import.meta.dirname, name)

  it('discovers rules across family directories, sorted by slug', () => {
    expect(discovery.discoverRuleSlugs(fixture('valid'))).toMatchSnapshot()
  })

  it('returns the memoized result on a second call', () => {
    support.expectMemoized(discovery.discoverRuleSlugs, fixture('valid'))
  })

  it('indexes discovered rules by slug', () => {
    const dir = fixture('valid')
    support.expectSlugIndex(discovery.discoverRuleIndex, discovery.discoverRuleSlugs, dir)
  })

  it('collects pages outside a family directory as strays', () => {
    expect(discovery.discoverRules(fixture('stray-page')).strayPages).toEqual(['loose.md'])
  })

  it('rejects bad-caption', () => {
    expect(() => discovery.discoverRules(fixture('bad-caption'))).toThrow(/invalid or missing caption/)
  })
})
