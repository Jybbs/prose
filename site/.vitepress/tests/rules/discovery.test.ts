import { discoverRules, discoverRuleSlugs } from '../../lib/rules/discovery'
import { fixtureDir }                       from '../support'

describe('discoverRules', () => {
  const fixture = (name: string): string => fixtureDir(import.meta.dirname, name)

  it('discovers rules across family directories, sorted by slug', () => {
    expect(discoverRuleSlugs(fixture('valid'))).toMatchSnapshot()
  })

  it('returns the memoized result on a second call', () => {
    const dir = fixture('valid')
    expect(discoverRuleSlugs(dir)).toBe(discoverRuleSlugs(dir))
  })

  it('collects pages outside a family directory as strays', () => {
    expect(discoverRules(fixture('stray-page')).strayPages).toEqual(['loose.md'])
  })

  it('rejects bad-caption', () => {
    expect(() => discoverRules(fixture('bad-caption'))).toThrow(/invalid or missing caption/)
  })
})
