// @vitest-environment happy-dom
import { flushPromises, mount } from '@vue/test-utils'

import SandboxCodeEditor from '../../theme/components/sandbox/SandboxCodeEditor.vue'

vi.mock('../../lib/sandbox/highlight', () => import('../highlight-stub'))

const mountEditor = (modelValue: string) =>
  mount(SandboxCodeEditor, { props: { lang: 'python', modelValue } })

describe('SandboxCodeEditor', () => {
  it('tracks the textarea\'s horizontal scroll on the highlight layer', async () => {
    const wrapper = mountEditor('x = 1')
    await flushPromises()

    const input = wrapper.get('textarea')
    input.element.scrollLeft = 120
    await input.trigger('scroll')
    expect(wrapper.get('div.code-editor-layer').element.scrollLeft).toBe(120)
  })

  it('pads a trailing newline so the layer measures the textarea\'s height', async () => {
    const wrapper = mountEditor('x = 1\n')
    await flushPromises()

    expect(wrapper.get('div.code-editor-layer').element.textContent).toBe('x = 1\n ')
  })
})
