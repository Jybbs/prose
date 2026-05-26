import { useElementBounding, useEventListener, useResizeObserver } from '@vueuse/core'
import { onMounted, ref, type Ref }                                from 'vue'

interface CarouselMeasurement {
  fits      : Ref<boolean>
  halfWidth : Ref<number>
  measure   : () => void
}

export function useCarouselMeasurement(
  trackRef      : Readonly<Ref<HTMLElement | null>>,
  viewportRef   : Readonly<Ref<HTMLElement | null>>,
  originalCount : () => number
): CarouselMeasurement {
  const { width: vpWidth } = useElementBounding(viewportRef)
  const halfWidth          = ref(0)
  const fits               = ref(false)

  function measure() {
    const track = trackRef.value
    if (!track) return
    const count = originalCount()
    const cards = track.children
    if (cards.length >= count) {
      const firstCard  = cards[0]         as HTMLElement
      const lastCard   = cards[count - 1] as HTMLElement
      const trackStyle = getComputedStyle(track)
      const gap        = parseFloat(trackStyle.columnGap)    || 0
      const padLeft    = parseFloat(trackStyle.paddingLeft)  || 0
      const padRight   = parseFloat(trackStyle.paddingRight) || 0
      const span       = lastCard.offsetLeft + lastCard.offsetWidth - firstCard.offsetLeft
      halfWidth.value  = span + gap
      fits.value       = vpWidth.value > 0 && span + padLeft + padRight <= vpWidth.value
    }
    else {
      halfWidth.value = 0
      fits.value      = false
    }
  }

  useResizeObserver(viewportRef, measure)
  useEventListener('resize', measure)
  onMounted(async () => {
    if ('fonts' in document) await document.fonts.ready
    measure()
  })

  return { fits, halfWidth, measure }
}
