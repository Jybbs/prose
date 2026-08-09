// @vitest-environment happy-dom
import { mount } from '@vue/test-utils'

import Surfaces             from '../../theme/components/landing/surfaces/Surfaces.vue'
import { expectAccessible } from '../axe'
import { domTest }          from '../dom'
import { popperStubMount }  from '../popper-stub'

vi.mock('vitepress', () => ({ useRoute: () => ({ path: '/' }) }))

vi.mock('../../lib/landing/landing.data', () => ({
  data: {
    surfaces: [
      {
        bodyNodes : [{ kind: 'text', text: 'Columns line up around an ' },
                     { kind: 'term', slug: 'atomic', text: 'atomic' },
                     { kind: 'text', text: ' value.' }],
        family    : 'alignment',
        number    : 'I'
      },
      {
        bodyNodes : [{ kind: 'text', text: 'Siblings sort.' }],
        family    : 'ordering',
        number    : 'II'
      }
    ],
    workflow: []
  }
}))

vi.mock('../../lib/glossary/glossary.data', () => ({
  data: {
    entries: {
      atomic: {
        aliases        : [],
        definitionHtml : '<p>An indivisible literal.</p>',
        href           : '/reference/glossary#atomic',
        slug           : 'atomic'
      }
    }
  }
}))

vi.mock('../../lib/rules/rules.data', () => ({
  data: {
    byFamily : { alignment: [{ href: '/rules/alignment/align-equals', slug: 'align-equals' }] },
    list     : [{ href: '/rules/alignment/align-equals', slug: 'align-equals' }]
  }
}))

const mountSurfaces = () => mount(Surfaces, { global: popperStubMount })

describe('Surfaces', () => {
  domTest('doubles the track and hides the second copy from assistive tech', ({ reducedMotion }) => {
    reducedMotion(true)
    const cards = mountSurfaces().findAll('.surface-card')
    expect(cards).toHaveLength(4)
    expect(cards.map(c => c.attributes('data-family')))
      .toEqual(['alignment', 'ordering', 'alignment', 'ordering'])
    expect(cards.map(c => c.attributes('aria-hidden')))
      .toEqual([undefined, undefined, 'true', 'true'])
  })

  domTest('leaves every link in the hidden copy out of the tab order', ({ reducedMotion }) => {
    reducedMotion(true)
    const cards      = mountSurfaces().findAll('.surface-card')
    const tabindexes = (card: (typeof cards)[number]): (string | undefined)[] =>
      card.findAll('a').map(a => a.attributes('tabindex'))

    expect(tabindexes(cards[0]).every(t => t === undefined)).toBe(true)
    expect(tabindexes(cards[2]).length).toBeGreaterThan(0)
    expect(tabindexes(cards[2]).every(t => t === '-1')).toBe(true)
  })

  domTest('drops the hidden copy\'s glossary anchors out of the tab order too', ({ reducedMotion }) => {
    reducedMotion(true)
    const cards = mountSurfaces().findAll('.surface-card')
    expect(cards[0].get('.glossary-anchor').attributes('tabindex')).toBe('0')
    expect(cards[2].get('.glossary-anchor').attributes('tabindex')).toBe('-1')
  })

  domTest('counts the families and rules in the heading', ({ reducedMotion }) => {
    reducedMotion(true)
    expect(mountSurfaces().get('h2').text()).toBe('2 rule families. 1 rules.')
  })

  domTest('links each family card to its rules', ({ reducedMotion }) => {
    reducedMotion(true)
    const first = mountSurfaces().findAll('.surface-card')[0]
    expect(first.get('.surface-card-cover-link').attributes('href')).toBe('/rules/alignment/')
    expect(first.get('.surface-key').attributes('href')).toBe('/rules/alignment/align-equals')
  })

  domTest('renders with no axe violations', async ({ reducedMotion }) => {
    reducedMotion(true)
    await expectAccessible(mountSurfaces().html())
  })
})
