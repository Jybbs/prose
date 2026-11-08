import markdownit from 'markdown-it'

import { inlineNodes, type InlineNode } from './inline-nodes'

const md = markdownit()

// Parses inline Markdown with no plugin registered, so the nodes hold text,
// code, and elements such as emphasis and links, and never a glossary, rule,
// or primitive.
export function bareInlineNodes(src: string): InlineNode[] {
  return inlineNodes(md, src)
}

// Flattens inline Markdown to the text a reader sees, keeping the words inside
// each code span, emphasis, and link and dropping the marks and URLs around them.
export function inlineText(src: string): string {
  return bareInlineNodes(src).map(nodeText).join('')
}

// Returns the text one node reads as, a rule as its slug and a primitive as
// its display name.
export function nodeText(node: InlineNode): string {
  return node.kind === 'el'        ? node.children.map(nodeText).join('')
       : node.kind === 'rule'      ? node.slug
       : node.kind === 'primitive' ? node.display
       : node.text
}
