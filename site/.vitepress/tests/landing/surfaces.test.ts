// @vitest-environment happy-dom
import { mount } from '@vue/test-utils'

import Surfaces             from '../../theme/components/landing/surfaces/Surfaces.vue'
import { expectAccessible } from '../axe'
import { domTest }          from '../dom'

vi.mock('../../data/landing.data', () => ({
  data: {
    surfaces: [
      { bodyHtml: 'Columns line up.', family: 'alignment', number: 'I'  },
      { bodyHtml: 'Siblings sort.',   family: 'ordering',  number: 'II' }
    ],
    workflow: []
  }
}))

vi.mock('../../data/rules.data', () => ({
  data: {
    byFamily : { alignment: [{ href: '/rules/alignment/align-equals', slug: 'align-equals' }] },
    list     : [{ href: '/rules/alignment/align-equals', slug: 'align-equals' }]
  }
}))

describe('Surfaces', () => {
  domTest('doubles the track and inerts the second copy', ({ reducedMotion }) => {
    reducedMotion(true)
    const cards = mount(Surfaces).findAll('.surface-card')
    expect(cards).toHaveLength(4)
    expect(cards.map(c => c.attributes('data-family')))
      .toEqual(['alignment', 'ordering', 'alignment', 'ordering'])
    expect(cards.map(c => 'inert' in c.attributes())).toEqual([false, false, true, true])
  })

  domTest('counts the families and rules in the heading', ({ reducedMotion }) => {
    reducedMotion(true)
    expect(mount(Surfaces).get('h2').text()).toBe('2 rule families. 1 rules.')
  })

  domTest('links each family card to its rules', ({ reducedMotion }) => {
    reducedMotion(true)
    const first = mount(Surfaces).findAll('.surface-card')[0]
    expect(first.get('.surface-card-cover-link').attributes('href')).toBe('/rules/alignment/')
    expect(first.get('.tab').attributes('href')).toBe('/rules/alignment/align-equals')
  })

  domTest('renders with no axe violations', async ({ reducedMotion }) => {
    reducedMotion(true)
    await expectAccessible(mount(Surfaces).html())
  })
})
