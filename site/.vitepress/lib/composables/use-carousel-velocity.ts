import { clamp, useElementBounding, useRafFn } from '@vueuse/core'
import { ref, watchEffect, type Ref }          from 'vue'

import { MS_PER_SEC } from '../shared/constants'
import { posMod }     from '../shared/pos-mod'

interface CarouselVelocityOptions {
  baseSpeedPxPerSec : number
  edgeMarginPx      : number
  magnetGain        : number
  maxPullPxPerSec   : number
  reducedMotion     : Ref<boolean>
}

interface CarouselVelocity {
  offset         : Ref<number>
  onPointerLeave : () => void
  onPointerMove  : (event: PointerEvent) => void
}

export function useCarouselVelocity(
  viewportRef : Readonly<Ref<HTMLElement | null>>,
  halfWidth   : Readonly<Ref<number>>,
  fits        : Readonly<Ref<boolean>>,
  options     : CarouselVelocityOptions
): CarouselVelocity {
  const { left: vpLeft, right: vpRight } = useElementBounding(viewportRef)
  const offset = ref(0)
  let velocity = options.baseSpeedPxPerSec

  useRafFn(({ delta }) => {
    if (halfWidth.value > 0 && !options.reducedMotion.value && !fits.value) {
      offset.value = posMod(offset.value + velocity * delta / MS_PER_SEC, halfWidth.value)
    }
  }, { immediate: true })

  // A track that fits rests at its origin, and a track that overflows keeps
  // its offset within the remeasured half width.
  watchEffect(() => {
    if (fits.value) offset.value = 0
    else if (halfWidth.value > 0) offset.value = posMod(offset.value, halfWidth.value)
  })

  function onPointerLeave() {
    velocity = options.baseSpeedPxPerSec
  }

  function onPointerMove(event: PointerEvent) {
    if (fits.value) return
    const node = (event.target as HTMLElement).closest('.surface-card') as HTMLElement | null
    if (!node) {
      velocity = 0
      return
    }
    const cardRect = node.getBoundingClientRect()
    const leftGap  = cardRect.left  - vpLeft.value  - options.edgeMarginPx
    const rightGap = vpRight.value  - cardRect.right - options.edgeMarginPx
    const gap = leftGap < 0 ? leftGap : rightGap < 0 ? -rightGap : 0
    velocity  = clamp(gap * options.magnetGain, -options.maxPullPxPerSec, options.maxPullPxPerSec)
  }

  return { offset, onPointerLeave, onPointerMove }
}
