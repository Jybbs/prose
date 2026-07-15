// @vitest-environment happy-dom
import { flushPromises, mount } from '@vue/test-utils'

import CopyButton from '../../theme/components/base/CopyButton.vue'

// The stub covers what the button wires, the label, source, and the `copied`
// flag, leaving the real clipboard write to vueuse and the browser sandbox.
const { copySpy } = vi.hoisted(() => ({ copySpy: vi.fn<(value: string) => void>() }))

vi.mock('@vueuse/core', async () => {
  const { ref } = await import('vue')
  return {
    useClipboard: () => {
      const copied = ref(false)
      return { copied, copy: (value: string) => { copySpy(value); copied.value = true } }
    }
  }
})

describe('CopyButton', () => {
  it('labels the button, copies the source, and flags copied on click', async () => {
    const wrapper = mount(CopyButton, {
      props: { label: 'Copy prose.toml', source: 'code-line-length = 40' }
    })

    const button = wrapper.get('button.copy')
    expect(button.attributes('title')).toBe('Copy prose.toml')
    expect(button.attributes('aria-label')).toBe('Copy prose.toml')

    await button.trigger('click')
    await flushPromises()
    expect(copySpy).toHaveBeenCalledWith('code-line-length = 40')
    expect(button.classes()).toContain('copied')
    expect(button.attributes('title')).toBe('Copied')
  })
})
