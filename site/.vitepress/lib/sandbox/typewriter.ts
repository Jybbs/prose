import { codeHighlighter } from '../markdown/highlighter'
import { commonPrefix }    from '../shared/common-prefix'
import { SHIKI_THEMES }    from '../shared/constants'

const CARET = '<span class="code-caret" aria-hidden="true"></span>'

type Token = { content: string, style: string }

export type TokenLine = readonly Token[]

// One side of a typing run, its lines, the width of its widest changed
// line, and where its changed middle ends.
export interface TypingSide {
  lines  : string[]
  max    : number
  midEnd : number
}

// The line diff a typing run animates: the untouched prefix, each side's
// changed middle, and the shared character floor a single-line change
// sweeps from.
export interface TypingPlan {
  cur    : TypingSide
  floor  : number
  next   : TypingSide
  prefix : number
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

// One frame of a typing run, the held lines rendered whole and each line of
// the changed middle truncated to `chars` under its own caret.
export function typingFrame(
  tokens : readonly TokenLine[],
  side   : TypingSide,
  prefix : number,
  chars  : number
): { html: string, text: string } {
  const parts: string[] = []
  const texts: string[] = []
  side.lines.forEach((line, index) => {
    const held    = index < prefix || index >= side.midEnd
    const visible = held ? Number.POSITIVE_INFINITY : Math.min(chars, line.length)
    parts.push(lineHtml(tokens[index] ?? [], visible) + (held ? '' : CARET))
    texts.push(line.slice(0, visible))
  })
  return { html: parts.join('\n'), text: texts.join('\n') }
}

export function typingPlan(current: string, next: string): TypingPlan {
  const curLines  = current.split('\n')
  const nextLines = next.split('\n')
  const prefix    = commonPrefix(curLines, nextLines)
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
    const before = curMidEnd  > prefix ? curLines[prefix]  : ''
    const after  = nextMidEnd > prefix ? nextLines[prefix] : ''
    floor = commonPrefix(before, after)
  }
  return {
    cur    : { lines: curLines,  max: curMax,  midEnd: curMidEnd },
    floor  : floor,
    next   : { lines: nextLines, max: nextMax, midEnd: nextMidEnd },
    prefix : prefix
  }
}

function escapeHtml(text: string): string {
  return text.replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;')
}

function tokenStyle(style: Record<string, string> | undefined): string {
  return Object.entries(style ?? {}).map(([key, value]) => `${key}:${value}`).join(';')
}
