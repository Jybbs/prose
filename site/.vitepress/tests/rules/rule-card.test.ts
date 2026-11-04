// @vitest-environment happy-dom
import { mount } from '@vue/test-utils'

import RuleCard              from '../../theme/components/rules/RuleCard.vue'
import { rendersAccessibly } from '../axe'
import { rulesDataStub }     from '../rules-data-stub'

const { data } = rulesDataStub([{ family: 'layout', slug: 'reflow-calls' }])

const card = (clickable = true) => mount(RuleCard, {
  props: { clickable, rule: data.bySlug['reflow-calls'] }
})

describe('RuleCard', () => {
  it('heads the card at its right end with the family\'s nib, a middle dot, and its name', () => {
    const mark = card().get('.rule-card-mark')
    expect(mark.get('use').attributes('href')).toBe('#nib-basket-small')
    expect(mark.get('svg').attributes('aria-hidden')).toBe('true')
    expect(mark.get('.middle-dot').text()).toBe('·')
    expect(mark.get('.kicker').text()).toBe('Layout')
  })

  it('sets the slug and the caption in the card\'s body', () => {
    const body = card().get('.rule-card-body')
    expect(body.get('.rule-card-slug').attributes('title')).toBe('reflow-calls')
    expect(body.get('.rule-card-caption').text()).toBe('What reflow-calls does.')
  })

  it('covers a clickable card with a link to its rule page', () => {
    expect(card().get('.rule-card-cover').attributes('href')).toBe('/rules/layout/reflow-calls')
  })

  it('leaves the hover card a popper shows with no link of its own', () => {
    expect(card(false).find('.rule-card-cover').exists()).toBe(false)
  })

  rendersAccessibly(() => card().html())
})
