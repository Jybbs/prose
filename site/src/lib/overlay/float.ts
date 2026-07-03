import { autoUpdate, computePosition, flip, offset, shift } from '@floating-ui/dom'
import type { Placement }                                   from '@floating-ui/dom'

interface FloatOptions {
  gapPx     ?: number
  placement ?: Placement
}

// Pins `panel` against `anchor`, `flip` and `shift` keeping it inside the
// viewport while the page scrolls or resizes. Returns the cleanup that stops
// the tracking.
export function float(anchor: Element, panel: HTMLElement, options: FloatOptions = {}): () => void {
  const { gapPx = 8, placement = 'top' } = options
  return autoUpdate(anchor, panel, async () => {
    const { x, y } = await computePosition(anchor, panel, {
      middleware : [offset(gapPx), flip(), shift({ padding: 8 })],
      placement
    })
    panel.style.left = `${x}px`
    panel.style.top  = `${y}px`
  })
}
