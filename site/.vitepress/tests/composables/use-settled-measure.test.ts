// @vitest-environment happy-dom
import { flushPromises, mount }    from '@vue/test-utils'
import { defineComponent, h, ref } from 'vue'

import { useSettledMeasure } from '../../lib/composables/use-settled-measure'

class FakeResizeObserver {
  static latest: FakeResizeObserver | undefined
  callback: ResizeObserverCallback

  constructor(callback: ResizeObserverCallback) {
    this.callback = callback
    FakeResizeObserver.latest = this
  }

  disconnect(): void {}
  observe(): void {}
  unobserve(): void {}

  resize(): void {
    this.callback([], this as unknown as ResizeObserver)
  }
}

const mountWith = (measure: () => void) => {
  const target = ref(document.createElement('div'))
  return mount(defineComponent({
    setup() {
      useSettledMeasure(target, measure)
      return () => h('div')
    }
  }))
}

describe('useSettledMeasure', () => {
  it('measures once the fonts settle and again on each resize', async () => {
    vi.stubGlobal('ResizeObserver', FakeResizeObserver)
    const measure = vi.fn<() => void>()
    mountWith(measure)
    await flushPromises()
    expect(measure).toHaveBeenCalledTimes(1)
    FakeResizeObserver.latest!.resize()
    expect(measure).toHaveBeenCalledTimes(2)
    vi.unstubAllGlobals()
  })
})
