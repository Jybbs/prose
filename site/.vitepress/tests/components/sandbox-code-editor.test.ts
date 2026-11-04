// @vitest-environment happy-dom
import { flushPromises, mount } from '@vue/test-utils'

import SandboxCodeEditor from '../../theme/components/sandbox/SandboxCodeEditor.vue'

vi.mock('../../lib/shared/highlight', () => import('../highlight-stub'))

const mountEditor = (modelValue: string) =>
  mount(SandboxCodeEditor, { props: { lang: 'python', modelValue } })

describe('SandboxCodeEditor', () => {
  it('tracks the textarea\'s horizontal scroll on the highlight layer', async () => {
    const wrapper = mountEditor('x = 1')
    await flushPromises()

    const input = wrapper.get('textarea')
    input.element.scrollLeft = 120
    await input.trigger('scroll')
    expect(wrapper.get('div.sandbox-code-editor-layer').element.scrollLeft).toBe(120)
  })

  it('calls a parent keydown listener once for each key the textarea takes', async () => {
    const onKeydown = vi.fn<(event: KeyboardEvent) => void>()
    const wrapper   = mount(SandboxCodeEditor, {
      attrs : { onKeydown },
      props : { lang: 'python', modelValue: '' }
    })
    await wrapper.get('textarea').trigger('keydown', { key: 'Escape' })
    expect(onKeydown).toHaveBeenCalledOnce()
  })

  it('pads a trailing newline so the layer measures the textarea\'s height', async () => {
    const wrapper = mountEditor('x = 1\n')
    await flushPromises()

    expect(wrapper.get('div.sandbox-code-editor-layer').element.textContent).toBe('x = 1\n ')
  })
})
