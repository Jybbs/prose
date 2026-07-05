import * as discovery from '../../lib/rules/discovery'
import { fixtureDir } from '../support'

describe('discoverRules', () => {
  const fixture = (name: string): string => fixtureDir(import.meta.dirname, name)

  it('discovers rules across family directories, sorted by slug', () => {
    expect(discovery.discoverRuleSlugs(fixture('valid'))).toMatchSnapshot()
  })

  it('returns the memoized result on a second call', () => {
    const dir = fixture('valid')
    expect(discovery.discoverRuleSlugs(dir)).toBe(discovery.discoverRuleSlugs(dir))
  })

  it('indexes discovered rules by slug', () => {
    const dir = fixture('valid')
    expect([...discovery.discoverRuleIndex(dir).keys()]).toEqual(discovery.discoverRuleSlugs(dir).map(r => r.slug))
    expect(discovery.discoverRuleIndex(dir)).toBe(discovery.discoverRuleIndex(dir))
  })

  it('collects pages outside a family directory as strays', () => {
    expect(discovery.discoverRules(fixture('stray-page')).strayPages).toEqual(['loose.md'])
  })

  it('rejects bad-caption', () => {
    expect(() => discovery.discoverRules(fixture('bad-caption'))).toThrow(/invalid or missing caption/)
  })
})
