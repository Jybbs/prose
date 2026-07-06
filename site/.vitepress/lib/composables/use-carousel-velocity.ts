import { useElementBounding, useRafFn } from '@vueuse/core'
import { ref, type Ref }                from 'vue'

const MS_PER_SEC = 1000

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

  function wrap(value: number): number {
    if (halfWidth.value <= 0) return value
    return ((value % halfWidth.value) + halfWidth.value) % halfWidth.value
  }

  useRafFn(({ delta }) => {
    if (halfWidth.value > 0 && !options.reducedMotion.value && !fits.value) {
      offset.value = wrap(offset.value + velocity * delta / MS_PER_SEC)
    }
  }, { immediate: true })

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
    let v = 0
    if (leftGap < 0) {
      v = leftGap * options.magnetGain
    }
    else if (rightGap < 0) {
      v = -rightGap * options.magnetGain
    }
    velocity = Math.max(-options.maxPullPxPerSec, Math.min(options.maxPullPxPerSec, v))
  }

  return { offset, onPointerLeave, onPointerMove }
}
