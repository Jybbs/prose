import { tileStamps } from '../../lib/landing/hero-stamps'

describe('tileStamps', () => {
  it('emits one big plus four small stamps per cell', () => {
    expect(tileStamps(2, 3)).toHaveLength(2 * 3 * 5)
  })

  it('centers the first big stamp in its cell', () => {
    expect(tileStamps(2, 2)[0]).toEqual({ kind: 'big', rotate: -180, x: 25, y: 100 })
  })

  it('seeds the four corner letters from a fixed permutation per cell', () => {
    const smalls = tileStamps(1, 1).filter(s => s.kind === 'small')
    expect(smalls.map(s => s.letter)).toEqual(['r', 'o', 's', 'e'])
  })

  it('offsets the four small stamps to the cell corners', () => {
    const smalls = tileStamps(1, 1).filter(s => s.kind === 'small')
    expect(smalls.map(s => [s.x, s.y])).toEqual([[86, 28], [86, 172], [14, 172], [14, 28]])
  })

  it('picks a distinct permutation for an adjacent column', () => {
    const secondCell = tileStamps(2, 1).filter(s => s.kind === 'small').slice(4)
    expect(secondCell.map(s => s.letter)).toEqual(['s', 'o', 'e', 'r'])
  })

  it('is deterministic for the same dimensions', () => {
    expect(tileStamps(3, 3)).toEqual(tileStamps(3, 3))
  })
})
