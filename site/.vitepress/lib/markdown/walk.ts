export function walkBodyInlines<T extends { children: T[] | null; type: string }>(
  state : { tokens: T[] },
  visit : (block: T, children: T[]) => void
): void {
  for (let i = 0; i < state.tokens.length; i++) {
    const block = state.tokens[i]
    if (block.type !== 'inline' || !block.children) continue
    const prev = state.tokens[i - 1]
    if (prev?.type.startsWith('heading_')) continue
    visit(block, block.children)
  }
}
