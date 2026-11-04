import type { MarkdownIt } from 'markdown-it'

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

// Sets each operator of a code span outside a link in a `code-op` span.
export function codeOperatorPlugin(md: MarkdownIt): void {
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
