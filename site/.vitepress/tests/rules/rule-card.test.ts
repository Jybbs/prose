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
  it('sets the family badge alone on the tint at the card\'s left, named for a screen reader', () => {
    const mark = card().get('.rule-card-mark')
    expect(mark.classes()).toContain('nib-ground')
    expect(mark.get('use').attributes('href')).toBe('#nib-basket-small')
    expect(mark.get('svg').attributes()).toMatchObject({ 'aria-label': 'Layout', role: 'img' })
  })

  it('stacks the slug over the caption beside the mark', () => {
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
