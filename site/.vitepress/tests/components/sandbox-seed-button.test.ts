// @vitest-environment happy-dom
import { flushPromises, mount } from '@vue/test-utils'
import { renderToString }       from 'vue/server-renderer'
import { createSSRApp, h }      from 'vue'

import { decodeShare }       from '../../lib/sandbox/share-link'
import { SandboxSeedButton } from '../../theme/components/sandbox/SandboxSeedButton'

const props = {
  configHtml : '<pre><code>code-line-length = 40</code></pre>',
  inputHtml  : '<pre><code>x = 1</code></pre>'
}

describe('SandboxSeedButton', () => {

  it('links to the sandbox on a payload carrying the case source and config', async () => {
    const wrapper = mount(SandboxSeedButton, { props })
    await expect.poll(() => wrapper.find('a.sandbox-seed').exists()).toBe(true)

    const link = wrapper.get('a.sandbox-seed')
    expect(link.attributes('aria-label')).toBe('Open this case in the sandbox')
    const href = link.attributes('href') ?? ''
    expect(href).toMatch(/^\/sandbox\/#1\./)
    await expect(decodeShare(href.slice('/sandbox/'.length)))
      .resolves.toStrictEqual({ configToml: 'code-line-length = 40\n', source: 'x = 1\n' })
  })

  it('renders the same empty placeholder on the server as on the first client render', async () => {
    const server = await renderToString(createSSRApp({ render: () => h(SandboxSeedButton, props) }))
    expect(server).toBe('<!---->')
  })

  it('renders nothing where the platform lacks the compression codec', async () => {
    // oxlint-disable-next-line unicorn/no-useless-undefined -- `null` clears the typeof guard
    vi.stubGlobal('CompressionStream', undefined)
    const wrapper = mount(SandboxSeedButton, { props })
    await flushPromises()
    expect(wrapper.find('a.sandbox-seed').exists()).toBe(false)
  })
})
