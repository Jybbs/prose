// @vitest-environment happy-dom
import { ref } from 'vue'

import { useCarouselVelocity }     from '../../lib/composables/use-carousel-velocity'
import { mountSetup, rectElement } from '../dom'

const OPTIONS = { baseSpeedPxPerSec: 100, edgeMarginPx: 10, magnetGain: 2, maxPullPxPerSec: 400 }

const surfaceCard = (left: number, right: number): HTMLElement => {
  const el = rectElement({ left, right })
  el.classList.add('surface-card')
  return el
}

const pointerOn = (el: HTMLElement): PointerEvent => ({ target: el } as unknown as PointerEvent)

let frame: FrameRequestCallback | undefined

const step = (timestamp: number): void => frame?.(timestamp)

const mountVelocity = (halfWidth: number, fits = false, reducedMotion = false) => {
  const viewport = rectElement({ left: 0, right: 500 })
  return mountSetup(() => useCarouselVelocity(ref(viewport), ref(halfWidth), ref(fits), {
    ...OPTIONS,
    reducedMotion: ref(reducedMotion)
  }))
}

beforeEach(() => {
  frame = undefined
  vi.stubGlobal('requestAnimationFrame', (cb: FrameRequestCallback) => {
    frame = cb
    return 1
  })
  vi.stubGlobal('cancelAnimationFrame', () => {})
})

afterEach(() => {
  vi.unstubAllGlobals()
})

describe('useCarouselVelocity', () => {
  it('drifts forward at the base speed', () => {
    const api = mountVelocity(1000)
    step(1000)
    step(2000)
    expect(api.offset.value).toBe(100)
  })

  it('wraps the offset into one copy width', () => {
    const api = mountVelocity(150)
    step(1000)
    step(2000)
    step(3000)
    expect(api.offset.value).toBe(50)
  })

  it('holds still under reduced motion', () => {
    const api = mountVelocity(1000, false, true)
    step(1000)
    step(2000)
    expect(api.offset.value).toBe(0)
  })

  it('holds still and ignores the pointer when the track fits', () => {
    const api = mountVelocity(1000, true)
    api.onPointerMove(pointerOn(surfaceCard(-20, 180)))
    step(1000)
    step(2000)
    expect(api.offset.value).toBe(0)
  })

  it('halts when the pointer rests between cards', () => {
    const api = mountVelocity(1000)
    api.onPointerMove(pointerOn(document.createElement('div')))
    step(1000)
    step(2000)
    expect(api.offset.value).toBe(0)
  })

  it('magnet-pulls backward toward a card past the left margin', () => {
    const api = mountVelocity(1000)
    api.onPointerMove(pointerOn(surfaceCard(-20, 180)))
    step(1000)
    step(2000)
    expect(api.offset.value).toBe(940)
  })

  it('magnet-pulls forward toward a card past the right margin', () => {
    const api = mountVelocity(1000)
    api.onPointerMove(pointerOn(surfaceCard(320, 520)))
    step(1000)
    step(2000)
    expect(api.offset.value).toBe(60)
  })

  it('clamps the pull to the configured maximum', () => {
    const api = mountVelocity(1000)
    api.onPointerMove(pointerOn(surfaceCard(-300, -100)))
    step(1000)
    step(2000)
    expect(api.offset.value).toBe(600)
  })

  it('resumes the base drift after the pointer leaves', () => {
    const api = mountVelocity(1000)
    api.onPointerMove(pointerOn(document.createElement('div')))
    step(1000)
    step(2000)
    api.onPointerLeave()
    step(3000)
    expect(api.offset.value).toBe(100)
  })
})
