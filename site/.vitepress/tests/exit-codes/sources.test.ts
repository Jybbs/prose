import { LINT_CODE, SOURCES, exitCodes, exitCodesFrom } from '../../lib/exit-codes/sources'
import { repoRoot }                                     from '../../lib/shared/paths'

const HELP = `Usage: prose [OPTIONS] <COMMAND>

Exit codes:
  0    Clean: no findings, no rewrite pending
  1    Format would change: at least one rewrite is pending
  2    Lint violation: at least one lint finding was reported
  3    Parse error: input is not valid Python for the target version
  4    Config error: the config, the arguments, a path, or a rewrite failed validation
`

describe('SOURCES', () => {
  it('gives each exit code one entry', () => {
    const codes = SOURCES.map(source => source.code)
    expect(codes).toStrictEqual([...new Set(codes)].toSorted((a, b) => a - b))
  })

  it('names a code the loader can append the shipped-lint roster to', () => {
    expect(SOURCES.filter(source => source.code === LINT_CODE)).toHaveLength(1)
  })

  it.each(SOURCES.map(source => [source.code, source] as const))(
    'carries at least one detail line for code %i',
    (_code, source) => {
      expect(source.detail.length).toBeGreaterThan(0)
    }
  )
})

describe('exitCodesFrom', () => {
  it('reads each label and summary off its matrix row', () => {
    expect(exitCodesFrom(HELP).find(entry => entry.code === 3)).toMatchObject({
      label   : 'Parse error',
      summary : 'Input is not valid Python for the target version.'
    })
  })

  it('throws where the matrix prints no row for a code', () => {
    expect(() => exitCodesFrom(HELP.replace(/^ {2}4 .*$/m, ''))).toThrow(/no exit-code row for 4/)
  })
})

describe('exitCodes', () => {
  it('labels every code from the built binary', () => {
    const entries = exitCodes(repoRoot(import.meta.url))
    expect(entries.map(entry => entry.code)).toStrictEqual(SOURCES.map(source => source.code))
    for (const entry of entries) {
      expect(entry.label.length).toBeGreaterThan(0)
      expect(entry.summary).toMatch(/^[A-Z].*\.$/)
    }
  })
})
