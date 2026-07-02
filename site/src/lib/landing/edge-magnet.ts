import type { EmblaCarouselType } from 'embla-carousel'
import type { AutoScrollType }    from 'embla-carousel-auto-scroll'

export interface EdgeMagnetOptions {
  edgeMarginPx    : number
  magnetGain      : number
  maxPullPxPerSec : number
}

const MS_PER_FRAME = 1000 / 60

// The pointer-pull layer over the auto-scroll plugin. A pointer over a card
// hanging past either viewport edge pulls the scroll toward that card, speed
// scaled by the overshoot, while a pointer parked between cards pauses the
// crawl. The pull installs its own scroll body, the same seek contract the
// plugin's body fulfils, and hands the engine back to the plugin on release.
export function attachEdgeMagnet(
  embla      : EmblaCarouselType,
  autoScroll : AutoScrollType,
  options    : EdgeMagnetOptions
): void {
  const viewport = embla.rootNode()

  let engaged  = false
  let velocity = 0

  function magnetBody(): ReturnType<EmblaCarouselType['internalEngine']>['scrollBody'] {
    const engine = embla.internalEngine()
    const { index, location, previousLocation, scrollTarget, target } = engine
    let bodyVelocity = 0
    const noop = () => body
    const body = {
      direction : () => Math.sign(bodyVelocity),
      duration  : () => -1,
      seek      : () => {
        previousLocation.set(location)
        bodyVelocity = velocity * MS_PER_FRAME / 1000
        location.add(bodyVelocity)
        target.set(location)
        const currentIndex = scrollTarget.byDistance(0, false).index
        if (index.get() !== currentIndex) {
          index.set(currentIndex)
          embla.emit('select')
        }
        return body
      },
      settled         : () => false,
      useBaseDuration : noop,
      useBaseFriction : noop,
      useDuration     : noop,
      useFriction     : noop,
      velocity        : () => bodyVelocity
    }
    return body
  }

  function engage(): void {
    if (engaged) return
    engaged = true
    autoScroll.stop()
    const engine = embla.internalEngine()
    engine.scrollBody = magnetBody()
    engine.animation.start()
  }

  function release(): void {
    if (!engaged) return
    engaged  = false
    velocity = 0
    autoScroll.play(0)
  }

  function onPointerMove(event: PointerEvent): void {
    const card = (event.target as HTMLElement).closest('.surface-card')
    if (card === null) {
      velocity = 0
      engage()
      return
    }
    const viewportRect = viewport.getBoundingClientRect()
    const cardRect     = card.getBoundingClientRect()
    const leftGap      = cardRect.left - viewportRect.left - options.edgeMarginPx
    const rightGap     = viewportRect.right - cardRect.right - options.edgeMarginPx
    let pull = 0
    if (leftGap < 0)       pull = -leftGap * options.magnetGain
    else if (rightGap < 0) pull = rightGap * options.magnetGain
    if (pull === 0) {
      velocity = 0
      engage()
      return
    }
    velocity = Math.max(-options.maxPullPxPerSec, Math.min(options.maxPullPxPerSec, pull))
    engage()
  }

  viewport.addEventListener('pointermove', onPointerMove)
  viewport.addEventListener('pointerleave', release)
}
