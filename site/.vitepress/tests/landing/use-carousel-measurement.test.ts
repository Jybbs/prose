// @vitest-environment happy-dom
import { flushPromises } from '@vue/test-utils'
import { ref }           from 'vue'

import { useCarouselMeasurement }           from '../../theme/components/landing/surfaces/use-carousel-measurement'
import { domTest, mountSetup, rectElement } from '../dom'

const fakeCard = (offsetLeft: number, offsetWidth: number): HTMLElement => {
  const el = document.createElement('div')
  Object.defineProperty(el, 'offsetLeft',  { configurable: true, value: offsetLeft })
  Object.defineProperty(el, 'offsetWidth', { configurable: true, value: offsetWidth })
  return el
}

const mountMeasurement = (track: HTMLElement, viewport: HTMLElement, count: number) =>
  mountSetup(() => useCarouselMeasurement(ref(track), ref(viewport), () => count))

describe('useCarouselMeasurement', () => {
  it('holds zero until a full copy of the cards is in the track', async () => {
    const track = document.createElement('div')
    track.append(fakeCard(0, 200))
    const api = mountMeasurement(track, rectElement({ width: 500 }), 2)
    await flushPromises()
    expect(api.halfWidth.value).toBe(0)
    expect(api.fits.value).toBe(false)
  })

  it('adds the column gap to the copy width and the padding to the fit check', async () => {
    const track = document.createElement('div')
    track.append(fakeCard(0, 200), fakeCard(220, 200))
    const spy = vi.spyOn(window, 'getComputedStyle').mockReturnValue(
      { columnGap: '20px', paddingLeft: '30px', paddingRight: '10px' } as CSSStyleDeclaration
    )
    const api = mountMeasurement(track, rectElement({ width: 440 }), 2)
    await flushPromises()
    spy.mockRestore()
    expect(api.halfWidth.value).toBe(440)
    expect(api.fits.value).toBe(false)
  })

  it('reports no fit when the copy overflows the viewport', async () => {
    const track = document.createElement('div')
    track.append(fakeCard(0, 200), fakeCard(220, 200))
    const api = mountMeasurement(track, rectElement({ width: 300 }), 2)
    await flushPromises()
    expect(api.fits.value).toBe(false)
  })

  domTest('remeasures when the observed box changes', async ({ resizeObserver }) => {
    const track = document.createElement('div')
    const last  = fakeCard(220, 200)
    track.append(fakeCard(0, 200), last)
    const api = mountMeasurement(track, rectElement({ width: 500 }), 2)
    await flushPromises()
    expect(api.halfWidth.value).toBe(420)
    expect(api.fits.value).toBe(true)
    Object.defineProperty(last, 'offsetLeft', { configurable: true, value: 520 })
    resizeObserver.fire()
    expect(api.halfWidth.value).toBe(720)
    expect(api.fits.value).toBe(false)
  })
})
