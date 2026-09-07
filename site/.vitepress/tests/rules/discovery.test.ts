import * as discovery from '../../lib/rules/discovery'
import { fixtureDir } from '../support'

describe('discoverRules', () => {
  const fixture = (name: string): string => fixtureDir(import.meta.dirname, name)

  it('discovers rules across family directories, sorted by slug', () => {
    expect(discovery.discoverRuleSlugs(fixture('valid'))).toMatchSnapshot()
  })

  it('indexes discovered rules by slug', () => {
    const dir = fixture('valid')
    expect([...discovery.discoverRuleIndex(dir).keys()])
      .toEqual(discovery.discoverRuleSlugs(dir).map(r => r.slug))
  })

  it('collects pages outside a family directory as strays', () => {
    expect(discovery.discoverRules(fixture('stray-page')).strayPages).toEqual(['loose.md'])
  })

  it('rejects bad-caption', () => {
    expect(() => discovery.discoverRules(fixture('bad-caption'))).toThrow(/invalid or missing caption/)
  })
})
