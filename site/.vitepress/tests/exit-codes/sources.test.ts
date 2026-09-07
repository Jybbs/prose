import { LINT_CODE, SOURCES } from '../../lib/exit-codes/sources'

describe('SOURCES', () => {
  it('gives each exit code one entry', () => {
    const codes = SOURCES.map(source => source.code)
    expect(codes).toEqual([...new Set(codes)].toSorted((a, b) => a - b))
  })

  it('names a code the loader can append the shipped-lint roster to', () => {
    expect(SOURCES.filter(source => source.code === LINT_CODE)).toHaveLength(1)
  })

  it.each(SOURCES.map(source => [source.code, source] as const))(
    'carries a label, a summary, and at least one detail line for code %i',
    (_code, source) => {
      expect(source.label.length).toBeGreaterThan(0)
      expect(source.summary.length).toBeGreaterThan(0)
      expect(source.detail.length).toBeGreaterThan(0)
    }
  )
})
