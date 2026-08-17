// @vitest-environment happy-dom
import { flushPromises, mount } from '@vue/test-utils'

import { decodeShare }   from '../../lib/sandbox/share-link'
import SandboxSeedButton from '../../theme/components/sandbox/SandboxSeedButton.vue'

const seed = { configToml: 'code-line-length = 40\n', source: 'x = 1\n' }

describe('SandboxSeedButton', () => {
  afterEach(() => { vi.unstubAllGlobals() })

  it('links to the sandbox on a payload carrying the case source and config', async () => {
    const wrapper = mount(SandboxSeedButton, { props: { seed } })
    await vi.waitFor(() => expect(wrapper.find('a.sandbox-seed').exists()).toBe(true))

    const link = wrapper.get('a.sandbox-seed')
    expect(link.attributes('aria-label')).toBe('Open this case in the sandbox')
    const href = link.attributes('href') ?? ''
    expect(href).toMatch(/^\/sandbox\/#1\./)
    expect(await decodeShare(href.slice('/sandbox/'.length))).toEqual(seed)
  })

  it('renders nothing where the platform lacks the compression codec', async () => {
    // oxlint-disable-next-line unicorn/no-useless-undefined -- `null` clears the typeof guard
    vi.stubGlobal('CompressionStream', undefined)
    const wrapper = mount(SandboxSeedButton, { props: { seed } })
    await flushPromises()
    expect(wrapper.find('a.sandbox-seed').exists()).toBe(false)
  })
})
