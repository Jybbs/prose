import type { MarkdownIt, StateCore, Token } from 'markdown-it'

export interface CodePart {
  op   : boolean
  text : string
}

// Matches text that closes each quote it opens and holds no `#` outside a quote,
// as the text before `in` does in `"a" in b` but not in `f"{a} in {b}"` or `# a, in b`.
const CODE_PREFIX = /^(?:[^"'#]|"(?:\\.|[^"\\])*"|'(?:\\.|[^'\\])*')*$/

// Matches each operator the alignment rules pad, where it stands between spaces or
// at either end of a code span, so `x = 1`, `#`, `a == b`, and `x is not None` match
// while `name=value`, `a[1:2]`, a URL, and the `in` of a `for` clause do not. The
// third group holds a word operator, which `codeParts` sets only where `CODE_PREFIX`
// matches the text before it and the span does not end on a period.
const OPERATOR = /(^|\s)((is not|not in|is|(?<!\bfor\b.*)in)|==|!=|<=|>=|\*\*=|\/\/=|>>=|<<=|[-+*/%@&|^]=|=|<|>|:|#)(?=\s|$)/g

// Matches the punctuation a code span's chip holds on its own line.
const CHIP_TRAIL = /^[,.;:!?)\]}'"’”…]+/

// Pairs each closing token that can end on a code span with the token opening it.
const OPENERS: Record<string, string> = { em_close: 'em_open', link_close: 'link_open', strong_close: 'strong_open' }

// Splits a code span's text into its operators and the runs between them.
export function codeParts(text: string): CodePart[] {
  const parts = [] as CodePart[]
  let at      = 0
  for (const match of text.matchAll(OPERATOR)) {
    const start = match.index + match[1]!.length
    if (match[3] && (text.endsWith('.') || !CODE_PREFIX.test(text.slice(0, start)))) continue
    if (start > at) parts.push({ op: false, text: text.slice(at, start) })
    parts.push({ op: true, text: match[2]! })
    at = start + match[2]!.length
  }
  if (at < text.length || parts.length === 0) parts.push({ op: false, text: text.slice(at) })
  return parts
}

// Sets each operator of a code span outside a link in a `code-op` span, and each
// code span with the punctuation after it in a `code-unit` span.
export function codeOperatorPlugin(md: MarkdownIt): void {
  md.use(codeUnitPlugin)
  const plain = md.renderer.rules.code_inline!
  md.renderer.rules.code_inline = (tokens, idx, options, env, self) => {
    if (linked(tokens, idx)) return plain(tokens, idx, options, env, self)
    const token = tokens[idx]!
    const inner = codeParts(token.content)
      .map(({ op, text }) => op ? `<span class="code-op">${md.utils.escapeHtml(text)}</span>` : md.utils.escapeHtml(text))
      .join('')
    return `<code${self.renderAttrs(token)}>${inner}</code>`
  }
}

// Sets each code span with the punctuation after it in a `code-unit` span, which the
// HTML renderer and `inlineNodes` each read as they read any other element. The check
// reads the function's name, since each data loader bundles its own copy of this
// module and every copy applies the plugin to the one renderer VitePress caches.
export function codeUnitPlugin(md: MarkdownIt): void {
  if (!md.core.ruler.getRules('').some(({ name }) => name === setCodeUnits.name)) {
    md.core.ruler.push('code-unit', setCodeUnits)
  }
}

// Moves each code span, the emphasis and link closing on it, and the punctuation
// after them into one `code-unit` span. An emphasis opened earlier in the run closes
// ahead of the span and reopens inside it, so the span holds no other text. A code
// span inside a link that holds more than that span stays where it is.
function codeUnits(children: readonly Token[], Make: StateCore['Token']): Token[] {
  const tokens = [...children]
  const make   = (type: string, tag: string, nesting: 0 | 1 | -1) => new Make(type, tag, nesting)
  for (let at = tokens.length - 1; at >= 0; at--) {
    if (tokens[at]!.type !== 'code_inline') continue

    let end = at + 1
    while (tokens[end]?.type && tokens[end]!.type in OPENERS) end++
    const after = tokens[end]
    const tail  = after?.type === 'text' ? CHIP_TRAIL.exec(after.content)?.[0] : undefined
    if (!tail) continue

    const closers = tokens.slice(at + 1, end)
    let start     = at
    let matched   = 0
    while (matched < closers.length && tokens[start - 1]?.type === OPENERS[closers[matched]!.type]) {
      start--
      matched++
    }
    const outer = closers.slice(matched)
    if (outer.some(closer => closer.type === 'link_close') || linked(tokens, end)) continue

    const open = make('code_unit_open', 'span', 1)
    open.attrSet('class', 'code-unit')
    const text    = (content: string) => Object.assign(make('text', '', 0), { content })
    const rest    = after!.content.slice(tail.length)
    const opening = outer.map(({ tag, type }) => make(OPENERS[type]!, tag, 1)).toReversed()
    tokens.splice(
      start,
      end - start + 1,
      ...outer.map(({ tag, type }) => make(type, tag, -1)),
      open,
      ...opening,
      ...tokens.slice(start, end),
      text(tail),
      make('code_unit_close', 'span', -1),
      ...(rest ? [text(rest)] : [])
    )
  }
  return tokens
}

// Sets the `code-unit` spans of each inline block.
function setCodeUnits(state: StateCore): void {
  for (const block of state.tokens) {
    if (block.type === 'inline') block.children = codeUnits(block.children ?? [], state.Token)
  }
}

// Reports whether the token at `idx` sits inside a link its siblings open.
function linked(tokens: readonly { type: string }[], idx: number): boolean {
  let closed = 0
  for (let i = idx - 1; i >= 0; i--) {
    const { type } = tokens[i]!
    if (type === 'link_close') {
      closed++
    } else if (type === 'link_open') {
      if (!closed) return true
      closed--
    }
  }
  return false
}
