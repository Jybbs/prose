// @vitest-environment happy-dom
import { flushPromises } from '@vue/test-utils'
import { ref }           from 'vue'

import { useSettledMeasure }   from '../../lib/composables/use-settled-measure'
import { domTest, mountSetup } from '../dom'

describe('useSettledMeasure', () => {
  domTest('measures after the fonts settle and on resize', async ({ fonts, resizeObserver }) => {
    const measure = vi.fn<() => void>()
    mountSetup(() => useSettledMeasure(ref(document.createElement('div')), measure))
    await flushPromises()
    expect(measure).not.toHaveBeenCalled()
    fonts.settle()
    await flushPromises()
    expect(measure).toHaveBeenCalledTimes(1)
    resizeObserver.fire()
    expect(measure).toHaveBeenCalledTimes(2)
  })
})
