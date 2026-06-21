import { discoverPrimitives } from '../../lib/primitives/discovery'
import { discoverRuleSlugs }  from '../../lib/rules/discovery'
import { fixtureDir }         from '../support'

describe('discoverRuleSlugs', () => {
  const fixture = (name: string): string => fixtureDir('rules-discovery', name)

  it('discovers rules across family directories, sorted by slug', () => {
    expect(discoverRuleSlugs(fixture('valid'))).toMatchSnapshot()
  })

  it('returns the memoized result on a second call', () => {
    const dir = fixture('valid')
    expect(discoverRuleSlugs(dir)).toBe(discoverRuleSlugs(dir))
  })

  it.each([
    ['stray-page',       /must live in a family directory/],
    ['bad-caption',      /invalid or missing caption/],
    ['duplicate-slug',   /more than one family directory/],
    ['dangling-related', /lists invalid related slug/]
  ])('rejects %s', (dir, message) => {
    expect(() => discoverRuleSlugs(fixture(dir))).toThrow(message)
  })
})

describe('discoverPrimitives', () => {
  const fixture = (name: string): string => fixtureDir('primitives-discovery', name)

  it('discovers primitives sorted by filename', () => {
    expect(discoverPrimitives(fixture('valid'))).toMatchSnapshot()
  })

  it('returns the memoized result on a second call', () => {
    const dir = fixture('valid')
    expect(discoverPrimitives(dir)).toBe(discoverPrimitives(dir))
  })

  it.each([
    ['bad-consumes',      /invalid or missing consumes/],
    ['bad-layer',         /invalid or missing layer/],
    ['bad-stability',     /invalid or missing stability/],
    ['bad-summary',       /invalid or missing summary/],
    ['bad-tagline',       /invalid or missing tagline/],
    ['dangling-consumes', /consumes unknown primitive/],
    ['missing-h1',        /no H1 heading/]
  ])('rejects %s', (dir, message) => {
    expect(() => discoverPrimitives(fixture(dir))).toThrow(message)
  })
})
