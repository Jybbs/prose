import type MarkdownIt from 'markdown-it'

export function walkBodyInlines(
  state : { tokens: MarkdownIt.Token[] },
  visit : (block: MarkdownIt.Token) => void
): void {
  for (let i = 0; i < state.tokens.length; i++) {
    const block = state.tokens[i]
    if (block.type !== 'inline' || !block.children) continue
    const prev = state.tokens[i - 1]
    if (prev?.type.startsWith('heading_')) continue
    visit(block)
  }
}
