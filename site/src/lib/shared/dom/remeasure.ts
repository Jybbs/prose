// Wires a ResizeObserver on `target` and runs `measure` once the fonts
// settle, returning the observer so a caller can disconnect it.
export function remeasure(target: Element, measure: () => void): ResizeObserver {
  const observer = new ResizeObserver(measure)
  observer.observe(target)
  void document.fonts.ready.then(measure)
  return observer
}
