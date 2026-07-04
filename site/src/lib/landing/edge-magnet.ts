import type { EmblaCarouselType, ScrollBodyType } from 'embla-carousel'
import type { AutoScrollType }                    from 'embla-carousel-auto-scroll'
import * as fc                                    from 'fast-check'

import { closestFrom } from '../shared/dom/closest-from'

interface EdgeMagnetOptions {
  edgeMarginPx    : number
  magnetGain      : number
  maxPullPxPerSec : number
}

export const FRAMES_PER_SEC = 60

// The pointer-pull layer over the auto-scroll plugin. A pointer over a card
// hanging past either viewport edge pulls the scroll toward that card, speed
// scaled by the overshoot, while a pointer parked between cards pauses the
// crawl. The pull installs its own scroll body, the same seek contract the
// plugin's body fulfils, and hands the engine back to the plugin on release.
/* istanbul ignore next -- embla wiring, exercised only against a live carousel */
export function attachEdgeMagnet(
  embla      : EmblaCarouselType,
  autoScroll : AutoScrollType,
  options    : EdgeMagnetOptions
): void {
  const viewport = embla.rootNode()

  let engaged  = false
  let velocity = 0

  function magnetBody(): ScrollBodyType {
    const engine = embla.internalEngine()
    const { index, location, previousLocation, scrollTarget, target } = engine
    let bodyVelocity = 0
    const noop = () => body
    const body = {
      direction : () => Math.sign(bodyVelocity),
      duration  : () => -1,
      seek      : () => {
        previousLocation.set(location)
        bodyVelocity = velocity / FRAMES_PER_SEC
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
    const card = closestFrom(event, '.surface-card')
    if (card === null) {
      velocity = 0
      engage()
      return
    }
    velocity = edgePull(card.getBoundingClientRect(), options, viewport.getBoundingClientRect())
    engage()
  }

  viewport.addEventListener('pointermove', onPointerMove)
  viewport.addEventListener('pointerleave', release)
}

function edgePull(
  card     : { left: number, right: number },
  options  : EdgeMagnetOptions,
  viewport : { left: number, right: number }
): number {
  const { edgeMarginPx, magnetGain, maxPullPxPerSec } = options
  const leftGap  = card.left - viewport.left - edgeMarginPx
  const rightGap = viewport.right - card.right - edgeMarginPx
  const pull =
    leftGap  < 0 ? -leftGap  * magnetGain :
    rightGap < 0 ?  rightGap * magnetGain : 0
  return Math.max(-maxPullPxPerSec, Math.min(maxPullPxPerSec, pull))
}

if (import.meta.vitest) {
  const { describe, expect, test } = import.meta.vitest

  const OPTIONS  = { edgeMarginPx: 16, magnetGain: 0.5, maxPullPxPerSec: 800 }
  const VIEWPORT = { left: 0, right: 1000 }

  describe('edgePull', () => {
    test.each([
      { name: 'a card resting inside both edges pulls nothing',   card: { left: 100,   right: 300 },   pull: 0 },
      { name: 'a left overhang pulls toward the card',            card: { left: -40,   right: 160 },   pull: 28 },
      { name: 'a right overhang pulls the other way',             card: { left: 840,   right: 1040 },  pull: -28 },
      { name: 'a deep left overhang clamps to the speed cap',     card: { left: -2000, right: -1800 }, pull: 800 },
      { name: 'a deep right overhang clamps to the speed cap',    card: { left: 2800,  right: 3000 },  pull: -800 }
    ])('$name', ({ card, pull }) => {
      expect(edgePull(card, OPTIONS, VIEWPORT)).toBe(pull)
    })

    test('never pulls faster than the speed cap in either direction', () => {
      fc.assert(fc.property(fc.integer({ min: -5000, max: 5000 }), fc.integer({ min: 0, max: 400 }), (left, span) => {
        const magnitude = Math.abs(edgePull({ left, right: left + span }, OPTIONS, VIEWPORT))
        expect(magnitude).toBeLessThanOrEqual(OPTIONS.maxPullPxPerSec)
      }))
    })
  })
}
