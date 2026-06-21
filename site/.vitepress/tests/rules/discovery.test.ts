import { discoverRuleSlugs } from '../../lib/rules/discovery'
import { fixtureDir }        from '../support'

describe('discoverRuleSlugs', () => {
  const fixture = (name: string): string => fixtureDir(import.meta.dirname, name)

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
