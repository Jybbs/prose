const PATTERN  = /(?<![\w-])([Pp]rose)(?![\w-])/g
const SKIP_TAG = new Set(['A', 'CODE', 'PRE', 'SCRIPT', 'STYLE', 'TEXTAREA', 'INPUT', 'NOSCRIPT', 'TITLE'])
const MARK_CLS = 'prose-mark'

function wrapTextNode(node: Text): void {
  const text = node.nodeValue ?? ''
  if (!PATTERN.test(text)) {
    PATTERN.lastIndex = 0
    return
  }
  PATTERN.lastIndex = 0

  const frag = document.createDocumentFragment()
  let cursor = 0
  let match: RegExpExecArray | null
  while ((match = PATTERN.exec(text)) !== null) {
    if (match.index > cursor) {
      frag.appendChild(document.createTextNode(text.slice(cursor, match.index)))
    }
    const span = document.createElement('span')
    span.className   = MARK_CLS
    span.textContent = match[1]
    frag.appendChild(span)
    cursor = match.index + match[1].length
  }
  if (cursor < text.length) {
    frag.appendChild(document.createTextNode(text.slice(cursor)))
  }
  node.parentNode?.replaceChild(frag, node)
}

function walk(node: Node): void {
  if (node.nodeType === Node.TEXT_NODE) {
    wrapTextNode(node as Text)
    return
  }
  if (node.nodeType !== Node.ELEMENT_NODE) return
  const el = node as Element
  if (SKIP_TAG.has(el.tagName)) return
  if (el.classList.contains(MARK_CLS)) return
  for (const child of Array.from(el.childNodes)) walk(child)
}

export function applyProseMarks(): void {
  if (typeof window === 'undefined' || typeof document === 'undefined') return
  const root = document.querySelector<HTMLElement>('#app') ?? document.body
  walk(root)
}
