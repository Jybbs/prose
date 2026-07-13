import { codeHighlighter } from '../markdown/highlighter'
import { SHIKI_THEMES }    from '../shared/constants'

type Token = { content: string, style: string }

export type TokenLine = readonly Token[]

// The line diff a typing run animates: the untouched prefix and suffix,
// the changed middle's bounds and widths, and the shared character floor
// a single-line change sweeps from.
export interface TypingPlan {
  curLines   : string[]
  curMax     : number
  curMidEnd  : number
  floor      : number
  nextLines  : string[]
  nextMax    : number
  nextMidEnd : number
  prefix     : number
}

// The first `chars` characters of one tokenized line as styled spans.
export function lineHtml(line: TokenLine, chars: number): string {
  let remaining = chars
  let html      = ''
  for (const token of line) {
    if (remaining <= 0) break
    const slice = token.content.slice(0, remaining)
    html      += `<span style="${token.style}">${escapeHtml(slice)}</span>`
    remaining -= slice.length
  }
  return html
}

// Tokenizes `text` to per-line styled tokens through the shared client
// highlighter.
export async function tokenLines(text: string): Promise<TokenLine[]> {
  const highlighter = await codeHighlighter()
  const { tokens } = highlighter.codeToTokens(text, { lang: 'toml', themes: SHIKI_THEMES })
  return tokens.map(line => line.map(token => ({
    content : token.content,
    style   : tokenStyle(token.htmlStyle)
  })))
}

export function typingPlan(current: string, next: string): TypingPlan {
  const curLines  = current.split('\n')
  const nextLines = next.split('\n')
  let prefix = 0
  while (
    prefix < curLines.length && prefix < nextLines.length &&
    curLines[prefix] === nextLines[prefix]
  ) prefix += 1
  let suffix = 0
  while (
    suffix < curLines.length - prefix && suffix < nextLines.length - prefix &&
    curLines[curLines.length - 1 - suffix] === nextLines[nextLines.length - 1 - suffix]
  ) suffix += 1
  const curMidEnd  = curLines.length - suffix
  const nextMidEnd = nextLines.length - suffix
  const curMax     = Math.max(0, ...curLines.slice(prefix, curMidEnd).map(line => line.length))
  const nextMax    = Math.max(0, ...nextLines.slice(prefix, nextMidEnd).map(line => line.length))
  let floor = 0
  if (curMidEnd - prefix <= 1 && nextMidEnd - prefix <= 1) {
    const before = curMidEnd > prefix ? curLines[prefix] : ''
    const after  = nextMidEnd > prefix ? nextLines[prefix] : ''
    while (floor < before.length && floor < after.length && before[floor] === after[floor]) {
      floor += 1
    }
  }
  return { curLines, curMax, curMidEnd, floor, nextLines, nextMax, nextMidEnd, prefix }
}

function escapeHtml(text: string): string {
  return text.replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;')
}

function tokenStyle(style: Record<string, string> | undefined): string {
  return Object.entries(style ?? {}).map(([key, value]) => `${key}:${value}`).join(';')
}
