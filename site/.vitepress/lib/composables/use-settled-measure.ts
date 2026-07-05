import { useResizeObserver }             from '@vueuse/core'
import type { MaybeComputedElementRef }  from '@vueuse/core'
import { onMounted }                     from 'vue'

// Waits for the font swap as well as observing the box, because the swap
// changes glyph widths without resizing the observed container, so an
// observer alone would keep a first measure taken under fallback metrics.
export function useSettledMeasure(target: MaybeComputedElementRef, measure: () => void): void {
  useResizeObserver(target, measure)
  onMounted(async () => {
    if ('fonts' in document) await document.fonts.ready
    measure()
  })
}
