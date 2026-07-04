interface AxisOption {
  gloss : string
  id    : string
  mono  : string
}

export interface RenderedLine {
  anchor    : string | null
  anchorUbe : boolean
  countTint : CountTint | null
  text      : string
}

export const OUTCOMES = [
  { anchor: '🪻', args: 'check',         gloss: 'A clean run',       key: 'clean',  text: 'All clean.',                    tint: 'celadon' },
  { anchor: '🔖', args: 'check',         gloss: 'Violations found',  key: 'check',  text: '5 diagnostics in 2 files.',     tint: 'apricot' },
  { anchor: '🗞️', args: 'format',        gloss: 'Files reformatted', key: 'format', text: 'Reformatted 4 files.',          tint: 'apricot' },
  { anchor: '🗞️', args: 'format --diff', gloss: 'A diff preview',    key: 'diff',   text: '3 files would be reformatted.', tint: 'apricot' }
] as const

type Outcome   = (typeof OUTCOMES)[number]
type CountTint = Outcome['tint']

export const QUIET_OPTIONS: readonly AxisOption[] = [
  { gloss: 'full output', id: 'full',  mono: 'default' },
  { gloss: 'quiet',       id: 'quiet', mono: '--quiet' }
]

export const STREAM_OPTIONS: readonly AxisOption[] = [
  { gloss: 'on a tty', id: 'tty',     mono: 'interactive tty' },
  { gloss: 'piped',    id: 'pipe',    mono: '| cat'           },
  { gloss: 'no color', id: 'nocolor', mono: '--color never'   }
]

const outcomeFor = (id: string): Outcome => OUTCOMES.find(o => o.key === id) ?? OUTCOMES[0]

export function glossFor(outcomeId: string, quietId: string, streamId: string): string {
  const outcome = outcomeFor(outcomeId)
  const quiet   = QUIET_OPTIONS.find(q => q.id === quietId)   ?? QUIET_OPTIONS[0]
  const stream  = STREAM_OPTIONS.find(s => s.id === streamId) ?? STREAM_OPTIONS[0]
  return `${outcome.gloss}, ${quiet.gloss}, ${stream.gloss}.`
}

export function resolveSelection(
  outcomeId : string,
  quietId   : string,
  streamId  : string
): RenderedLine {
  const outcome = outcomeFor(outcomeId)
  return resolveLine(streamId === 'tty', outcome, quietId === 'quiet')
}

function resolveLine(colorBearing: boolean, outcome: Outcome, quiet: boolean): RenderedLine {
  if (quiet) return { anchor: null, anchorUbe: false, countTint: null, text: outcome.text }
  return {
    anchor    : outcome.anchor,
    anchorUbe : colorBearing,
    countTint : colorBearing ? outcome.tint : null,
    text      : outcome.text
  }
}

if (import.meta.vitest) {
  const { describe, expect, test } = import.meta.vitest

  describe('glossFor', () => {
    test.each([
      { name: 'combines the three axis glosses',       out: 'clean', quiet: 'quiet', stream: 'pipe', expected: 'A clean run, quiet, piped.'                    },
      { name: 'falls back to defaults on unknown ids', out: 'nope',  quiet: 'nope',  stream: 'nope', expected: `${OUTCOMES[0].gloss}, full output, on a tty.` }
    ])('$name', ({ out, quiet, stream, expected }) => {
      expect(glossFor(out, quiet, stream)).toBe(expected)
    })
  })

  describe('resolveSelection', () => {
    test.each([
      {
        name     : 'a quiet line drops the anchor and tint',
        out      : 'check', quiet: 'quiet', stream: 'tty',
        expected : { anchor: null, anchorUbe: false, countTint: null, text: '5 diagnostics in 2 files.' }
      },
      {
        name     : 'a tty line carries the anchor and tint',
        out      : 'check', quiet: 'full', stream: 'tty',
        expected : { anchor: '🔖', anchorUbe: true, countTint: 'apricot', text: '5 diagnostics in 2 files.' }
      },
      {
        name     : 'a piped line keeps the anchor but drops the tint',
        out      : 'clean', quiet: 'full', stream: 'pipe',
        expected : { anchor: '🪻', anchorUbe: false, countTint: null, text: 'All clean.' }
      },
      {
        name     : 'an unknown outcome falls back to the first',
        out      : 'nope', quiet: 'full', stream: 'tty',
        expected : { anchor: '🪻', anchorUbe: true, countTint: 'celadon', text: 'All clean.' }
      }
    ])('$name', ({ out, quiet, stream, expected }) => {
      expect(resolveSelection(out, quiet, stream)).toEqual(expected)
    })
  })
}
