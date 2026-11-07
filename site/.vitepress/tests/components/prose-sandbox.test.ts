// @vitest-environment happy-dom
import { mount } from '@vue/test-utils'

import { ProseSandbox } from '../../theme/components/sandbox/ProseSandbox'

vi.mock('../../lib/composables/use-prose-sandbox', async () => {
  const [{ fakeSandbox }, { ref }] = await Promise.all([import('../dom'), import('vue')])
  return { useProseSandbox: () => fakeSandbox({ configToml: ref('line-length = 100') }) }
})
vi.mock('../../lib/sandbox/config-schema.data', () => ({ data: {} }))
vi.mock('../../lib/sandbox/pool.data', () => ({ data: { cases: [] } }))

const mountSandbox = () => mount(ProseSandbox, {
  global: { stubs: { ProseSandboxControls: true, ProseSandboxSurface: true, ProseSandboxToml: true } }
})

describe('ProseSandbox', () => {
  it('marks its root while the surface reports an edit in progress', async () => {
    const wrapper = mountSandbox()
    expect(wrapper.attributes('data-editing')).toBeUndefined()
    await wrapper.findComponent({ name: 'ProseSandboxSurface' }).vm.$emit('update:editing', true)
    await wrapper.vm.$nextTick()
    expect(wrapper.attributes('data-editing')).toBe('true')
  })

  it('hides the copy, refresh, and link buttons while the surface reports an edit in progress', async () => {
    const wrapper = mountSandbox()
    await wrapper.findComponent({ name: 'ProseSandboxSurface' }).vm.$emit('update:editing', true)
    await wrapper.vm.$nextTick()
    const styles = wrapper.findAll('.sandbox-py > button').map(button => button.attributes('style'))
    expect(styles).toStrictEqual(Array.from({ length: 3 }, () => 'display: none;'))
  })

  it('arms the refresh button on the first click of a dirty refresh', async () => {
    const wrapper = mountSandbox()
    const refresh = wrapper.get('button[title="New example"]')
    expect(refresh.attributes('data-armed')).toBeUndefined()
    await refresh.trigger('click')
    expect(refresh.attributes('data-armed')).toBe('true')
  })
})
