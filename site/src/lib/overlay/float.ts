import { autoUpdate, computePosition, flip, offset, shift } from '@floating-ui/dom'
import type { ComputePositionReturn, Placement, Platform, ReferenceElement } from '@floating-ui/dom'

interface FloatOptions {
  gapPx     ?: number
  placement ?: Placement
  platform  ?: Platform
}

// Pins `panel` against `anchor`, keeping it inside the viewport while the page
// scrolls or resizes. Returns the cleanup that stops the tracking.
/* istanbul ignore next -- autoUpdate DOM wrapper, exercised only in a real browser */
export function float(anchor: Element, panel: HTMLElement, options: FloatOptions = {}): () => void {
  return autoUpdate(anchor, panel, async () => {
    const { x, y } = await positionPanel(anchor, panel, options)
    panel.style.left = `${x}px`
    panel.style.top  = `${y}px`
  })
}

// A caller-supplied `platform` overrides the default DOM one.
function positionPanel(
  anchor  : ReferenceElement,
  panel   : HTMLElement,
  options : FloatOptions = {}
): Promise<ComputePositionReturn> {
  const { gapPx = 8, placement = 'top', platform } = options
  return computePosition(anchor, panel, {
    middleware : [offset(gapPx), flip(), shift({ padding: 8 })],
    placement,
    ...(platform ? { platform } : {})
  })
}

if (import.meta.vitest) {
  const { describe, expect, test } = import.meta.vitest

  interface Rect {
    height : number
    width  : number
    x      : number
    y      : number
  }

  // A fake floating-ui platform feeding fabricated rects, so the real flip and
  // shift middleware runs in node with no browser.
  const stubPlatform = (reference: Rect, viewport: { height: number, width: number }): Platform => {
    const floating = { height: 150, width: 200, x: 0, y: 0 }
    return {
      getClippingRect : () => ({ x: 0, y: 0, ...viewport }),
      getDimensions   : () => ({ height: floating.height, width: floating.width }),
      getElementRects : () => ({ floating, reference })
    } as unknown as Platform
  }

  describe('positionPanel', () => {
    const anchor = {} as Element
    const panel  = {} as HTMLElement

    test('flips below the anchor when a top placement overflows the viewport top', async () => {
      const platform = stubPlatform({ height: 30, width: 100, x: 450, y: 5 }, { height: 800, width: 1000 })
      const { placement } = await positionPanel(anchor, panel, { placement: 'top', platform })
      expect(placement).toBe('bottom')
    })

    test('shifts the panel to keep it inside the right edge', async () => {
      const platform = stubPlatform({ height: 30, width: 100, x: 950, y: 400 }, { height: 800, width: 1000 })
      const { x } = await positionPanel(anchor, panel, { placement: 'bottom', platform })
      expect(x + 200).toBeLessThanOrEqual(1000)
    })

    test('defaults to the top placement when none is given', async () => {
      const platform = stubPlatform({ height: 30, width: 100, x: 450, y: 400 }, { height: 800, width: 1000 })
      const { placement } = await positionPanel(anchor, panel, { platform })
      expect(placement).toBe('top')
    })
  })
}
