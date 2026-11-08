// @vitest-environment happy-dom
import { mount } from '@vue/test-utils'

import { SectionGlance }     from '../../theme/components/reference/SectionGlance'
import { rendersAccessibly } from '../axe'

vi.mock('../../lib/reference/section-glance.data', () => ({
  data: {
    architecture: [{
      descriptionNodes : [{ kind: 'text', text: 'owns the original text.' }],
      href             : '/architecture/source',
      titleNodes       : [
        { kind: 'text', text: 'The ' },
        { kind: 'code', text: 'Source' },
        { kind: 'text', text: ' Type' }
      ]
    }]
  }
}))

describe('SectionGlance', () => {
  it('links each entry by its title ahead of its description', () => {
    const w = mount(SectionGlance, { props: { section: 'architecture' } })
    expect(w.get('li > a').attributes('href')).toBe('/architecture/source')
    expect(w.get('li > a strong code').text()).toBe('Source')
    expect(w.get('li').text()).toBe('The Source Type owns the original text.')
  })

  rendersAccessibly(() => mount(SectionGlance, { props: { section: 'architecture' } }).html())
})
