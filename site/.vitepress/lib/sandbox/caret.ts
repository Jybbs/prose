// The flat character offset under a point, measured as the text between the
// root's start and the caret position the browser resolves there.
export function offsetAt(root: HTMLElement, x: number, y: number): number {
  const position = document.caretPositionFromPoint?.(x, y)
  const caret    = position ? undefined : document.caretRangeFromPoint?.(x, y)
  const target   = position?.offsetNode ?? caret?.startContainer
  const inNode   = position?.offset ?? caret?.startOffset ?? 0
  if (!target || !root.contains(target)) return 0
  const range = document.createRange()
  range.setStart(root, 0)
  range.setEnd(target, inNode)
  return range.toString().length
}
