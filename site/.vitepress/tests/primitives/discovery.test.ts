import { discoverPrimitives } from '../../lib/primitives/discovery'
import { fixtureDir }         from '../support'

describe('discoverPrimitives', () => {
  const fixture = (name: string): string => fixtureDir(import.meta.dirname, name)

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
