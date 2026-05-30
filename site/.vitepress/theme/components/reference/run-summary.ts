type CountTint  = 'apricot' | 'celadon'
type OutcomeKey = 'check' | 'clean' | 'diff' | 'format'

interface AxisOption {
  gloss : string
  id    : string
  mono  : string
}

interface Outcome {
  anchor : string
  args   : string
  gloss  : string
  key    : OutcomeKey
  text   : string
  tint   : CountTint
}

export interface RenderedLine {
  anchor    : string | null
  anchorUbe : boolean
  countTint : CountTint | null
  text      : string
}

export interface SelectOption {
  id       : string
  mono     : string
  preview ?: RenderedLine
}

export const OUTCOMES: readonly Outcome[] = [
  { anchor: '🪻', args: 'check',         gloss: 'A clean run',       key: 'clean',  text: 'All clean.',                    tint: 'celadon' },
  { anchor: '🔖', args: 'check',         gloss: 'Violations found',  key: 'check',  text: '5 diagnostics in 2 files.',     tint: 'apricot' },
  { anchor: '🗞️', args: 'format',        gloss: 'Files reformatted', key: 'format', text: 'Reformatted 4 files.',          tint: 'apricot' },
  { anchor: '🗞️', args: 'format --diff', gloss: 'A diff preview',    key: 'diff',   text: '3 files would be reformatted.', tint: 'apricot' }
]

export const QUIET_OPTIONS: readonly AxisOption[] = [
  { gloss: 'full output', id: 'full',  mono: 'default' },
  { gloss: 'quiet',       id: 'quiet', mono: '--quiet' }
]

export const STREAM_OPTIONS: readonly AxisOption[] = [
  { gloss: 'on a tty', id: 'tty',     mono: 'interactive tty' },
  { gloss: 'piped',    id: 'pipe',    mono: '| cat'           },
  { gloss: 'no color', id: 'nocolor', mono: '--color never'   }
]

function resolveLine(outcome: Outcome, quiet: boolean, colorBearing: boolean): RenderedLine {
  if (quiet) return { anchor: null, anchorUbe: false, countTint: null, text: outcome.text }
  return {
    anchor    : outcome.anchor,
    anchorUbe : colorBearing,
    countTint : colorBearing ? outcome.tint : null,
    text      : outcome.text
  }
}

export function resolveSelection(outcomeId: string, quietId: string, streamId: string): RenderedLine {
  const outcome = OUTCOMES.find(o => o.key === outcomeId) ?? OUTCOMES[0]
  return resolveLine(outcome, quietId === 'quiet', streamId === 'tty')
}

export function glossFor(outcomeId: string, quietId: string, streamId: string): string {
  const outcome = OUTCOMES.find(o => o.key === outcomeId)     ?? OUTCOMES[0]
  const quiet   = QUIET_OPTIONS.find(q => q.id === quietId)   ?? QUIET_OPTIONS[0]
  const stream  = STREAM_OPTIONS.find(s => s.id === streamId) ?? STREAM_OPTIONS[0]
  return `${outcome.gloss}, ${quiet.gloss}, ${stream.gloss}.`
}
