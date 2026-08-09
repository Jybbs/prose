import { inertEnv } from './inert-env'

// The fields the walk reads off a parsed token, declared structurally so
// neither parser interface below couples to markdown-it's own type layout.
interface ParsedToken {
  attrs    : [string, string][] | null
  children : ParsedToken[] | null
  content  : string
  meta     : Record<string, unknown> | null
  nesting  : 0 | 1 | -1
  tag      : string
  type     : string
}

// Both `MarkdownIt` and VitePress's `MarkdownRenderer` satisfy these, and they
// disagree on the `highlight` option's return type.
export interface BlockParser {
  parse(src: string, env: object): ParsedToken[]
}

export interface InlineParser {
  parseInline(src: string, env: object): ParsedToken[]
}

export type InlineNode =
  | { kind : 'code',      text     : string }
  | { kind : 'el',        attrs    : Record<string, string>, children: InlineNode[], tag: string }
  | { kind : 'primitive', display  : string, slug: string }
  | { kind : 'rule',      slug     : string }
  | { kind : 'term',      slug     : string, text: string }
  | { kind : 'text',      text     : string }

// A leaf the walker cannot model would vanish silently from the rendered
// prose, so an unmapped token type fails the build instead.
function leaf(token: ParsedToken): InlineNode {
  switch (token.type) {
    case 'code_inline': return { kind: 'code', text: token.content }
    case 'softbreak':   return { kind: 'text', text: ' ' }
    case 'text':        return { kind: 'text', text: token.content }

    case 'doc_link':
      return token.meta?.kind === 'rule'
        ? { kind: 'rule', slug: token.content }
        : { kind: 'primitive', display: token.meta?.display as string, slug: token.content }

    case 'glossary_term':
      return { kind: 'term', slug: token.meta?.slug as string, text: token.content }

    default:
      throw new Error(`inlineNodes cannot map the "${token.type}" token`)
  }
}

export function blockNodes(md: BlockParser, src: string): InlineNode[] {
  return walk(md.parse(src, inertEnv()))
}

export function inlineNodes(md: InlineParser, src: string): InlineNode[] {
  return walk(md.parseInline(src, inertEnv())[0]?.children ?? [])
}

// A block parse nests its inline content one level down, under an `inline`
// token, whereas the open and close tokens around it nest like any other pair.
function walk(tokens: readonly ParsedToken[]): InlineNode[] {
  const root  : InlineNode[]   = []
  const stack : InlineNode[][] = [root]

  for (const token of tokens.flatMap(t => (t.type === 'inline' ? t.children ?? [] : [t]))) {
    const parent = stack.at(-1)!

    if (token.nesting === 1) {
      const el: InlineNode = {
        kind     : 'el',
        attrs    : Object.fromEntries(token.attrs ?? []),
        children : [],
        tag      : token.tag
      }
      parent.push(el)
      stack.push(el.children)
      continue
    }

    if (token.nesting === -1) {
      stack.pop()
      continue
    }

    // markdown-it leaves an empty text token on each side of an emphasis
    // delimiter run, which an HTML renderer swallows and a tree would not.
    if (token.type === 'text' && token.content === '') continue

    parent.push(leaf(token))
  }

  return root
}
