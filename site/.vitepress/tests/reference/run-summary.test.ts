import * as runSummary from '../../lib/reference/run-summary'

describe('resolveSelection', () => {
  it('renders an anchored, tinted line for a color-bearing tty run', () => {
    expect(runSummary.resolveSelection('check', 'full', 'tty')).toEqual({
      anchor: '🔖', anchorUbe: true, countTint: 'apricot', text: '5 diagnostics in 2 files.'
    })
  })

  it('drops the anchor and tint when quiet', () => {
    expect(runSummary.resolveSelection('check', 'quiet', 'tty')).toEqual({
      anchor: null, anchorUbe: false, countTint: null, text: '5 diagnostics in 2 files.'
    })
  })

  it('keeps the anchor but drops the tint when piped', () => {
    expect(runSummary.resolveSelection('clean', 'full', 'pipe')).toEqual({
      anchor: '🪻', anchorUbe: false, countTint: null, text: 'All clean.'
    })
  })

  it('falls back to the first outcome for an unknown id', () => {
    expect(runSummary.resolveSelection('bogus', 'full', 'tty')).toMatchObject({ text: runSummary.OUTCOMES[0].text })
  })
})

describe('glossFor', () => {
  it('composes the three axis glosses', () => {
    expect(runSummary.glossFor('check', 'quiet', 'pipe')).toBe('Violations found, quiet, piped.')
  })

  it('falls back to the first option of each axis for unknown ids', () => {
    expect(runSummary.glossFor('x', 'y', 'z')).toBe('A clean run, full output, on a tty.')
  })
})
