type CountTint  = 'apricot' | 'celadon'
type OutcomeKey = 'check' | 'clean' | 'diff' | 'format'

interface Outcome {
  anchor : string
  args   : string
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
  { anchor: '🪻', args: 'check',         key: 'clean',  text: 'All clean.',                    tint: 'celadon' },
  { anchor: '🔖', args: 'check',         key: 'check',  text: '5 diagnostics in 2 files.',     tint: 'apricot' },
  { anchor: '🗞️', args: 'format',        key: 'format', text: 'Reformatted 4 files.',          tint: 'apricot' },
  { anchor: '🗞️', args: 'format --diff', key: 'diff',   text: '3 files would be reformatted.', tint: 'apricot' }
]

export const QUIET_OPTIONS: readonly SelectOption[] = [
  { id: 'full',  mono: 'default' },
  { id: 'quiet', mono: '--quiet' }
]

export const STREAM_OPTIONS: readonly SelectOption[] = [
  { id: 'tty',     mono: 'interactive tty' },
  { id: 'pipe',    mono: '| cat'           },
  { id: 'nocolor', mono: '--color never'   }
]

const OUTCOME_GLOSS: Record<string, string> = {
  check  : 'Violations found',
  clean  : 'A clean run',
  diff   : 'A diff preview',
  format : 'Files reformatted'
}

const STREAM_GLOSS: Record<string, string> = {
  nocolor : 'no color',
  pipe    : 'piped',
  tty     : 'on a tty'
}

const VERBOSITY_GLOSS: Record<string, string> = {
  full  : 'full output',
  quiet : 'quiet'
}

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
  const outcome = OUTCOME_GLOSS[outcomeId]   ?? OUTCOME_GLOSS.clean
  const stream  = STREAM_GLOSS[streamId]     ?? STREAM_GLOSS.tty
  const quiet   = VERBOSITY_GLOSS[quietId]   ?? VERBOSITY_GLOSS.full
  return `${outcome}, ${quiet}, ${stream}.`
}
