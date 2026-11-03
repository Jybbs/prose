// @vitest-environment happy-dom
import { mount } from '@vue/test-utils'
import { ref }   from 'vue'

import ProseSandboxControls from '../../theme/components/sandbox/ProseSandboxControls.vue'
import { fakeSandbox }      from '../dom'

vi.mock('../../lib/rules/rules.data', async () => (await import('../rules-data-stub')).rulesDataStub())

const KNOB = { default: 88, key: 'code-line-length', label: 'Code' }

describe('ProseSandboxControls', () => {
  it('passes on the key and the hue of the knob a press holds', async () => {
    const wrapper = mount(ProseSandboxControls, {
      props: { sandbox: fakeSandbox({ lengthImpact: ref([KNOB.key]), lengths: [KNOB] }) }
    })
    const stop = wrapper.get('.sandbox-ruler-stop')
    stop.element.setPointerCapture = vi.fn<(pointerId: number) => void>()
    await stop.trigger('pointerdown', { button: 0, pointerId: 1 })
    const hue = stop.attributes('data-hue')
    expect(hue).toMatch(/\S/)
    expect(wrapper.emitted('dragging')).toStrictEqual([[KNOB.key, hue]])
  })
})
