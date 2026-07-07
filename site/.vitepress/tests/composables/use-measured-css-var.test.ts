// @vitest-environment happy-dom
import { flushPromises } from '@vue/test-utils'
import { ref }           from 'vue'

import { useMeasuredCssVar }             from '../../lib/composables/use-measured-css-var'
import { domTest, mountSetup, nextFrame } from '../dom'

describe('useMeasuredCssVar', () => {
  domTest('writes the measured width to the prop once the fonts settle', async ({ fonts }) => {
    const el     = document.createElement('div')
    const target = ref<HTMLElement | null>(el)
    mountSetup(() => useMeasuredCssVar({ measure: () => 42, propName: '--w', target }))
    fonts.settle()
    await flushPromises()
    await nextFrame()
    expect(el.style.getPropertyValue('--w')).toBe('42px')
  })

  domTest('leaves the prop unset when the measure returns null', async ({ fonts }) => {
    const el     = document.createElement('div')
    const target = ref<HTMLElement | null>(el)
    mountSetup(() => useMeasuredCssVar({ measure: () => null, propName: '--w', target }))
    fonts.settle()
    await flushPromises()
    await nextFrame()
    expect(el.style.getPropertyValue('--w')).toBe('')
  })

  domTest('re-measures when a trigger changes', async ({ fonts }) => {
    const el      = document.createElement('div')
    const target  = ref<HTMLElement | null>(el)
    const trigger = ref(0)
    const measure = vi.fn<() => number>(() => 10)
    mountSetup(() => useMeasuredCssVar({ measure, propName: '--w', target, triggers: [trigger] }))
    fonts.settle()
    await flushPromises()
    await nextFrame()
    const before = measure.mock.calls.length
    trigger.value = 1
    await flushPromises()
    await nextFrame()
    expect(measure.mock.calls.length).toBeGreaterThan(before)
  })
})
