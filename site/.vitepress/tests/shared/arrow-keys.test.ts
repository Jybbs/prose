import { arrowIndex } from '../../lib/shared/arrow-keys'

describe('arrowIndex', () => {
  it.each([
    ['ArrowDown',  2, 3],
    ['ArrowRight', 2, 3],
    ['ArrowUp',    2, 1],
    ['ArrowLeft',  2, 1],
    ['Home',       2, 0],
    ['End',        2, 4]
  ])('moves %s from %i to %i', (key, at, to) => {
    expect(arrowIndex(key, at, 5)).toBe(to)
  })

  it.each([['ArrowUp', 0, 0], ['ArrowDown', 4, 4]])('stops %s at the end from %i', (key, at, to) => {
    expect(arrowIndex(key, at, 5)).toBe(to)
  })

  it.each([['ArrowLeft', 0, 4], ['ArrowRight', 4, 0]])('wraps %s past the end from %i', (key, at, to) => {
    expect(arrowIndex(key, at, 5, true)).toBe(to)
  })

  it('returns undefined for a key it does not move by', () => {
    expect(arrowIndex('Enter', 2, 5)).toBeUndefined()
  })
})
