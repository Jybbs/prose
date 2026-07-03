// Creates an element carrying a class and optional text, the building block
// the island scripts assemble their panels from.
export function el<K extends keyof HTMLElementTagNameMap>(
  tag       : K,
  className : string,
  content  ?: string
): HTMLElementTagNameMap[K] {
  const node = document.createElement(tag)
  node.className = className
  if (content !== undefined) node.textContent = content
  return node
}
