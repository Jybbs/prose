import MarkdownIt from 'markdown-it'

import { inlineNodes, type InlineNode } from './inline-nodes'

const md = new MarkdownIt()

// Parses inline Markdown with no plugin registered, so the nodes hold text,
// code, and emphasis and never a glossary, rule, or primitive.
export function bareInlineNodes(src: string): InlineNode[] {
  return inlineNodes(md, src)
}

// Flattens inline Markdown to the text a reader sees, keeping the words inside
// each code span and emphasis and dropping the marks around them.
export function inlineText(src: string): string {
  return bareInlineNodes(src).map(nodeText).join('')
}

function nodeText(node: InlineNode): string {
  if (node.kind === 'el') return node.children.map(nodeText).join('')
  return 'text' in node ? node.text : ''
}
