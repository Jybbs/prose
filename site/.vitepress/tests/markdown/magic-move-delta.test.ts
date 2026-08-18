import { lineChurn } from '../../lib/markdown/magic-move-delta'

describe('lineChurn', () => {
  it('counts nothing when both states are identical', () => {
    expect(lineChurn('a\nb\nc', 'a\nb\nc')).toMatchObject({ churn: 0, shifts: 0 })
  })

  it('counts a respaced line as a shift rather than a rewrite', () => {
    // Its tokens move by transform, which composites far cheaper than the
    // enters and restyles a real content change pays for.
    expect(lineChurn('x = 1\ny = 2', 'x   = 1\ny   = 2')).toMatchObject({ churn: 0, shifts: 2 })
  })

  it('counts a line whose content changed as a rewrite', () => {
    expect(lineChurn('a\nb\nc', 'a\nB\nc')).toMatchObject({ churn: 1, shifts: 0 })
  })

  it('counts every line an insertion pushes out of place', () => {
    expect(lineChurn('a\nb\nc', 'x\na\nb\nc')).toMatchObject({ churn: 4 })
  })

  it('counts a pure append as the appended lines alone', () => {
    expect(lineChurn('a\nb', 'a\nb\nc')).toMatchObject({ churn: 1, shifts: 0 })
  })

  it('reads the same in both directions', () => {
    const forward  = lineChurn('a\nb\nc', 'a\nx\ny\nc')
    const backward = lineChurn('a\nx\ny\nc', 'a\nb\nc')
    expect(forward.churn).toBe(backward.churn)
    expect(forward.shifts).toBe(backward.shifts)
  })

  it('reports the line height of each state', () => {
    expect(lineChurn('a\nb', 'a\nb\nc\nd').lines).toEqual([2, 4])
  })
})
