// @vitest-environment happy-dom
import { mount } from '@vue/test-utils'
import { defineComponent } from 'vue'

import * as kit         from '../../kit'
import { declarations } from '../../kit/catalog'
import KitCatalog       from '../../kit/KitCatalog'

vi.mock('../../lib/composables/route', () => ({ useCurrentRule: () => ({ value: null }) }))

vi.mock('../../lib/shared/tools.data', () => ({
  data: { entries: { uv: { href: 'https://example.org', icon: { body: '', viewBox: '0 0 24 24' }, name: 'uv' } } }
}))

vi.mock('../../lib/kit/catalog.data', () => ({
  data: new Proxy({}, { get: (_, name) => ({ file: 'piece.tsx', html: `<b>${String(name)}</b>` }) })
}))

describe('kit catalog', () => {
  it('declares every export of the entry', () => {
    expect(Object.keys(kit).filter(name => !declarations.has(name))).toStrictEqual([])
  })

  it('declares only what the entry exports', () => {
    expect([...declarations.keys()].filter(name => !(name in kit))).toStrictEqual([])
  })

  it('gives each piece a use and an example', () => {
    for (const { examples, use } of declarations.values()) {
      expect(use).not.toBe('')
      expect(examples).not.toHaveLength(0)
    }
  })

  it.each([...declarations])('renders the examples of %s', (_name, { examples }) => {
    for (const { render } of examples) mount(defineComponent({ render }))
  })

  it('renders a section for each declaration beside its source', () => {
    const wrapper = mount(KitCatalog)
    expect(wrapper.findAll('section > h2').map(heading => heading.attributes('id')))
      .toStrictEqual([...declarations.keys()].map(name => name.toLowerCase()))
    expect(wrapper.find('section b').text()).toBe([...declarations.keys()][0])
  })

  it('leaves the example heading out of the page outline', () => {
    expect(mount(KitCatalog).findAll('h2.ignore-header').map(heading => heading.attributes('id')))
      .toStrictEqual(['doc-heading-example'])
  })
})
