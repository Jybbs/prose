export function walkBodyInlines<T extends { children: T[] | null, type: string }>(
  state : { tokens: T[] },
  visit : (block: T, children: T[]) => void
): void {
  for (const [i, block] of state.tokens.entries()) {
    if (block.type !== 'inline' || !block.children) continue
    if (state.tokens[i - 1]?.type.startsWith('heading_')) continue
    visit(block, block.children)
  }
}
