// @vitest-environment happy-dom
import { mount } from '@vue/test-utils'

import { Tool, ToolIcon } from '../../kit'

vi.mock('../../lib/shared/tools.data', () => ({
  data: {
    entries: {
      uv: {
        href : 'https://docs.astral.sh/uv/',
        icon : { body: '<path fill="currentColor" d="M0 0h24v24H0z"/>', viewBox: '0 0 24 24' },
        name : 'uv'
      }
    }
  }
}))

describe('Tool', () => {
  it('sets its icon and name in an outbound link', () => {
    const link = mount(Tool, { props: { slug: 'uv' } }).get('a.tool-mark')
    expect(link.attributes('href')).toBe('https://docs.astral.sh/uv/')
    expect(link.attributes('title')).toBe('uv')
    expect(link.attributes()).toMatchObject({ rel: 'noopener', target: '_blank' })
    expect(link.get('code').text()).toBe('uv')
    expect(link.get('code svg').attributes('viewBox')).toBe('0 0 24 24')
  })
})

describe('ToolIcon', () => {
  it('draws the icon alone, hidden from assistive technology', () => {
    const w = mount(ToolIcon, { attrs: { class: 'size-12' }, props: { slug: 'uv' } })
    expect(w.element.tagName.toLowerCase()).toBe('svg')
    expect(w.classes()).toContain('size-12')
    expect(w.attributes('aria-hidden')).toBe('true')
    expect(w.attributes('viewBox')).toBe('0 0 24 24')
    expect(w.text()).toBe('')
  })
})
