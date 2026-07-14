// @vitest-environment happy-dom
import { flushPromises, mount } from '@vue/test-utils'
import { promiseTimeout }       from '@vueuse/core'
import { nextTick, ref }        from 'vue'

import ProseSandboxToml      from '../../theme/components/sandbox/ProseSandboxToml.vue'
import type { ProseSandbox } from '../../lib/composables/use-prose-sandbox'
import { domTest }           from '../dom'

vi.mock('../../lib/sandbox/highlight', () => import('../highlight-stub'))

vi.mock('../../lib/markdown/highlighter', () => ({
  codeHighlighter: () => Promise.resolve({
    codeToTokens: (text: string) => ({
      tokens: text.split('\n').map(line => [{ content: line, htmlStyle: undefined }])
    })
  })
}))

const fakeSandbox = (configToml = ''): ProseSandbox => ({
  configError : ref(''),
  configToml  : ref(configToml)
} as unknown as ProseSandbox)

const mountToml = (sandbox: ProseSandbox) => mount(ProseSandboxToml, { props: { sandbox } })

// happy-dom leaves `isVisible` truthy for a `v-show`-hidden element, so
// visibility asserts on the inline style the directive writes.
const hidden = (wrapper: ReturnType<typeof mountToml>, selector: string): boolean =>
  wrapper.get(selector).attributes('style')?.includes('display: none') ?? false

describe('ProseSandboxToml', () => {
  domTest('types a config change and settles onto the target text', async ({ reducedMotion }) => {
    reducedMotion(false)
    const sandbox = fakeSandbox()
    const wrapper = mountToml(sandbox)
    await flushPromises()

    sandbox.configToml.value = 'code-line-length = 100'
    await vi.waitFor(() => {
      expect(wrapper.get('.sandbox-toml-display').html()).toContain('code-line-length = 100')
    })
    expect(hidden(wrapper, '.code-typewriter')).toBe(true)
  })

  domTest('abandons a stale run when a newer change lands mid-type', async ({ reducedMotion }) => {
    reducedMotion(false)
    const sandbox = fakeSandbox()
    const wrapper = mountToml(sandbox)
    await flushPromises()

    sandbox.configToml.value = 'rules.align-equals = false\nrules.blank-lines = false'
    await vi.waitFor(() => expect(hidden(wrapper, '.code-typewriter')).toBe(false))
    sandbox.configToml.value = 'code-line-length = 40'
    await vi.waitFor(() => {
      expect(hidden(wrapper, '.sandbox-toml-display')).toBe(false)
      expect(wrapper.get('.sandbox-toml-display').html()).toContain('code-line-length = 40')
    })
    expect(wrapper.get('.sandbox-toml-display').html()).not.toContain('align-equals')
  })

  domTest('abandons the run when the reader clicks in mid-type', async ({ reducedMotion }) => {
    reducedMotion(false)
    const sandbox = fakeSandbox()
    const wrapper = mountToml(sandbox)
    await flushPromises()

    sandbox.configToml.value = 'code-line-length = 100'
    await nextTick()
    await wrapper.get('.sandbox-toml-display').trigger('click')

    // The reader's own edit lands while editing, so its watch is spent before
    // the blur rather than firing a fresh run that would mask the stale one.
    sandbox.configToml.value = 'code-line-length = 60'
    await nextTick()
    await wrapper.get('textarea').trigger('blur')
    await promiseTimeout(600)
    await flushPromises()

    const display = wrapper.get('.sandbox-toml-display')
    expect(display.html()).toContain('code-line-length = 60')
    expect(display.html()).not.toContain('code-line-length = 100')
  })

  domTest('snaps straight to the settled text under reduced motion', async ({ reducedMotion }) => {
    reducedMotion(true)
    const sandbox = fakeSandbox()
    const wrapper = mountToml(sandbox)
    await flushPromises()

    sandbox.configToml.value = 'code-line-length = 60'
    await vi.waitFor(() => {
      expect(wrapper.get('.sandbox-toml-display').html()).toContain('code-line-length = 60')
    })
    expect(hidden(wrapper, '.code-typewriter')).toBe(true)
  })
})
