// @vitest-environment happy-dom
import { flushPromises, mount } from '@vue/test-utils'
import { promiseTimeout }       from '@vueuse/core'
import { nextTick, ref }        from 'vue'

import ProseSandboxToml                   from '../../theme/components/sandbox/ProseSandboxToml.vue'
import type { ProseSandbox }              from '../../lib/composables/use-prose-sandbox'
import { domTest, fakeSandbox, isHidden } from '../dom'

vi.mock('../../lib/shared/highlight', () => import('../highlight-stub'))

vi.mock('../../lib/markdown/highlighter', () => import('../highlighter-stub'))

const tomlSandbox = (configToml = '', configNotices: readonly string[] = []) =>
  fakeSandbox({ configNotices: ref(configNotices), configToml: ref(configToml) })

const mountToml = async (sandbox: ProseSandbox) => {
  const wrapper = mount(ProseSandboxToml, { props: { sandbox } })
  await flushPromises()
  return wrapper
}

describe('ProseSandboxToml', () => {
  domTest('marks the row and counts the key a notice names', async () => {
    const notice  = 'warning: unknown key `no-such-key` in [tool.prose]'
    const sandbox = tomlSandbox('code-line-length = 88\nno-such-key = 1', [notice])
    const wrapper = await mountToml(sandbox)

    expect(wrapper.get('.config-notice-strip').text()).toContain('1 unknown key')
    expect(wrapper.findAll('.sandbox-toml-gutter [data-flagged]')).toHaveLength(1)
    // The located notice reads off its own key rather than stacking below.
    expect(wrapper.find('.code-panel-warning').exists()).toBe(false)
  })

  domTest('keeps a notice the source cannot place beside the parse error', async () => {
    const sandbox = tomlSandbox('[this is not valid', ['warning: unknown key `absent`'])
    sandbox.configError.value = 'unexpected character'
    const wrapper = await mountToml(sandbox)

    expect(wrapper.get('.code-panel-error').text()).toContain('unexpected character')
    expect(wrapper.get('.code-panel-warning').text()).toContain('absent')
    expect(wrapper.find('.config-notice-strip').exists()).toBe(false)
  })

  domTest('types a config change and settles onto the target text', async ({ reducedMotion }) => {
    reducedMotion(false)
    const sandbox = tomlSandbox()
    const wrapper = await mountToml(sandbox)

    sandbox.configToml.value = 'code-line-length = 100'
    await expect.poll(() => wrapper.get('.sandbox-toml-display').html()).toContain('code-line-length = 100')
    expect(isHidden(wrapper.get('.code-typewriter'))).toBe(true)
  })

  domTest('abandons a stale run when a newer change lands mid-type', async ({ reducedMotion }) => {
    reducedMotion(false)
    const sandbox = tomlSandbox()
    const wrapper = await mountToml(sandbox)

    sandbox.configToml.value = 'rules.align-equals = false\nrules.space-statements = false'
    await expect.poll(() => isHidden(wrapper.get('.code-typewriter'))).toBe(false)
    sandbox.configToml.value = 'code-line-length = 40'
    await vi.waitFor(() => {
      expect(isHidden(wrapper.get('.sandbox-toml-display'))).toBe(false)
      expect(wrapper.get('.sandbox-toml-display').html()).toContain('code-line-length = 40')
    })
    expect(wrapper.get('.sandbox-toml-display').html()).not.toContain('align-equals')
  })

  domTest('abandons the run when the reader clicks in mid-type', async ({ reducedMotion }) => {
    reducedMotion(false)
    const sandbox = tomlSandbox()
    const wrapper = await mountToml(sandbox)

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
    const sandbox = tomlSandbox()
    const wrapper = await mountToml(sandbox)

    sandbox.configToml.value = 'code-line-length = 60'
    await flushPromises()

    expect(wrapper.get('.sandbox-toml-display').html()).toContain('code-line-length = 60')
    expect(isHidden(wrapper.get('.code-typewriter'))).toBe(true)
  })
})
