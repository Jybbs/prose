// @vitest-environment happy-dom
import { mount } from '@vue/test-utils'

import IntegrationCardGrid   from '../../theme/components/integrations/IntegrationCardGrid.vue'
import { rendersAccessibly } from '../axe'

vi.mock('../../lib/integrations/integrations.data', () => ({
  data: [
    {
      href         : '/integrations/editor',
      summaryNodes : [{ kind: 'text', text: 'Formats on save.' }],
      tagline      : 'Save',
      title        : 'Editor'
    }
  ]
}))

describe('IntegrationCardGrid', () => {
  it('links each card to its page by its title', () => {
    const w = mount(IntegrationCardGrid)
    expect(w.get('.int-card a').attributes('href')).toBe('/integrations/editor')
    expect(w.get('.int-card a').text()).toBe('Editor')
    expect(w.get('.int-body').text()).toBe('Formats on save.')
  })

  it('draws its footer in gray at rest', () => {
    const footer = mount(IntegrationCardGrid).get('.int-card > p:last-child')
    expect(footer.text()).toBe('Open →')
    expect(footer.classes()).toContain('text-text-2')
  })

  it.each(['hover', 'has-focus-visible'])('colors its edge and footer under %s', state => {
    const card = mount(IntegrationCardGrid).get('.int-card')
    expect(card.classes()).toContain(`${state}:border-ube`)
    expect(card.get('p:last-child').classes()).toContain(`group-${state}:text-link`)
  })

  rendersAccessibly(() => mount(IntegrationCardGrid).html())
})
