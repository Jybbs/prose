import { commonPrefix } from '../../lib/shared/common-prefix'

describe('commonPrefix', () => {
  it.each([
    ['abc', 'abd',  2],
    ['abc', 'abc',  3],
    ['',    'abc',  0],
    ['abc', 'xyz',  0],
    ['ab',  'abcd', 2]
  ])('counts the leading characters %j and %j share', (a, b, expected) => {
    expect(commonPrefix(a, b)).toBe(expected)
  })

  it('counts the leading entries two arrays share', () => {
    expect(commonPrefix(['a', 'b', 'c'], ['a', 'b', 'x'])).toBe(2)
  })
})
