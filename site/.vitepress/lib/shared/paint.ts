// Reads a CSS time such as `0.4s` or `400ms`, or the first of a list of them,
// as milliseconds.
export function millis(time: string): number {
  return parseFloat(time) * (time.split(',')[0]!.trim().endsWith('ms') ? 1 : 1000)
}

// Resolves two frames later, after the browser has painted the current
// DOM, so a class re-added on the next line fires its transition rather
// than batching away with the removal.
export function nextPaint(): Promise<void> {
  return new Promise(resolve => {
    requestAnimationFrame(() => requestAnimationFrame(() => resolve()))
  })
}

// Reads the curve a card's morph eases on, which `tokens.css` declares off
// the root element.
export function morphEase(): string {
  return getComputedStyle(document.documentElement).getPropertyValue('--prose-ease-morph').trim()
}

// Reports whether the reader has set the device to reduce motion.
export function reducesMotion(win: Window = window): boolean {
  return win.matchMedia('(prefers-reduced-motion: reduce)').matches
}

// Reads the squiggle-draw duration `tokens.css` declares off the root
// element, returning 0 where the custom property is absent.
export function ruleDrawMs(): number {
  const declared = getComputedStyle(document.documentElement).getPropertyValue('--prose-rule-draw-ms')
  return Number(declared)
}

// Writes `place` onto `el` with its transition dropped for that one write
// where `snap` is set, through `data-snap`, so the element appears in place
// rather than traveling there.
export function snapInPlace(el: HTMLElement, snap: boolean, place: () => void): void {
  if (snap) el.dataset.snap = ''
  place()
  if (!snap) return
  el.getBoundingClientRect()
  delete el.dataset.snap
}
