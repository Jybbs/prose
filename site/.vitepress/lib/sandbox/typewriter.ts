import { codeHighlighter } from '../markdown/highlighter'
import { commonPrefix }    from '../shared/common-prefix'
import { SHIKI_THEMES }    from '../shared/constants'
import { escapeHtml }      from '../shared/escape-html'

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

// Renders the first `chars` characters of one tokenized line as styled spans.
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
  const [{ stringifyTokenStyle }, highlighter] = await Promise.all([
    import('shiki/core'),
    codeHighlighter()
  ])
  const { tokens } = highlighter.codeToTokens(text, { lang: 'toml', themes: SHIKI_THEMES })
  return tokens.map(line => line.map(token => ({
    content : token.content,
    style   : stringifyTokenStyle(token.htmlStyle ?? {})
  })))
}

// Renders one frame of a typing run, holding the untouched lines whole and
// truncating each line of the changed middle to `chars` under its own caret.
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
  const suffix    = commonPrefix(
    curLines.slice(prefix).toReversed(),
    nextLines.slice(prefix).toReversed()
  )
  const curMidEnd  = curLines.length - suffix
  const nextMidEnd = nextLines.length - suffix
  const curMid     = curLines.slice(prefix, curMidEnd)
  const nextMid    = nextLines.slice(prefix, nextMidEnd)
  const curMax     = Math.max(0, ...curMid.map(line => line.length))
  const nextMax    = Math.max(0, ...nextMid.map(line => line.length))
  const floor      = curMid.length <= 1 && nextMid.length <= 1
    ? commonPrefix(curMid[0] ?? '', nextMid[0] ?? '')
    : 0
  return {
    cur    : { lines: curLines,  max: curMax,  midEnd: curMidEnd },
    floor  : floor,
    next   : { lines: nextLines, max: nextMax, midEnd: nextMidEnd },
    prefix : prefix
  }
}
