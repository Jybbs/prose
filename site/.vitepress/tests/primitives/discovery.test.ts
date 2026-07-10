import * as discovery from '../../lib/primitives/discovery'
import * as support   from '../support'

describe('discoverPrimitives', () => {
  const fixture = (name: string): string => support.fixtureDir(import.meta.dirname, name)

  it('discovers primitives sorted by filename', () => {
    expect(discovery.discoverPrimitives(fixture('valid'))).toMatchSnapshot()
  })

  it('returns the memoized result on a second call', () => {
    support.expectMemoized(discovery.discoverPrimitives, fixture('valid'))
  })

  it('indexes discovered primitives by slug', () => {
    const dir = fixture('valid')
    support.expectSlugIndex(discovery.discoverPrimitiveIndex, discovery.discoverPrimitives, dir)
  })

  it.each([
    ['bad-consumes',  /invalid or missing consumes/],
    ['bad-layer',     /invalid or missing layer/],
    ['bad-stability', /invalid or missing stability/],
    ['bad-summary',   /invalid or missing summary/],
    ['bad-tagline',   /invalid or missing tagline/],
    ['missing-h1',    /no H1 heading/]
  ])('rejects %s', (dir, message) => {
    expect(() => discovery.discoverPrimitives(fixture(dir))).toThrow(message)
  })
})
