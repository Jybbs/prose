import * as discovery from '../../lib/primitives/discovery'
import { fixtureDir } from '../support'

describe('discoverPrimitives', () => {
  const fixture = (name: string): string => fixtureDir(import.meta.dirname, name)

  it('discovers primitives sorted by filename', () => {
    expect(discovery.discoverPrimitives(fixture('valid'))).toMatchSnapshot()
  })

  it('returns the memoized result on a second call', () => {
    const dir = fixture('valid')
    expect(discovery.discoverPrimitives(dir)).toBe(discovery.discoverPrimitives(dir))
  })

  it('indexes discovered primitives by slug', () => {
    const dir = fixture('valid')
    expect([...discovery.discoverPrimitiveIndex(dir).keys()])
      .toEqual(discovery.discoverPrimitives(dir).map(p => p.slug))
    expect(discovery.discoverPrimitiveIndex(dir)).toBe(discovery.discoverPrimitiveIndex(dir))
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
