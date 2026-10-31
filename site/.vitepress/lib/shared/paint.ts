// Resolves two frames later, after the browser has painted the current
// DOM, so a class re-added on the next line fires its transition rather
// than batching away with the removal.
export function nextPaint(): Promise<void> {
  return new Promise(resolve => {
    requestAnimationFrame(() => requestAnimationFrame(() => resolve()))
  })
}

// Reads the squiggle-draw duration `tokens.css` declares off the root
// element, returning 0 where the custom property is absent.
export function ruleDrawMs(): number {
  const declared = getComputedStyle(document.documentElement).getPropertyValue('--prose-rule-draw-ms')
  return Number(declared)
}
