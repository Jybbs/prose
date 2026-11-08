// @vitest-environment happy-dom
import { mount } from '@vue/test-utils'

import { PipelineOrder } from '../../theme/components/rules/PipelineOrder'

vi.mock('../../lib/rules/rules.data', async () =>
  (await import('../rules-data-stub')).rulesDataStub([
    { family: 'layout',   slug: 'reflow-calls' },
    { family: 'ordering', slug: 'group-imports' }
  ]))

vi.mock('../../lib/rules/pipeline.data', () => ({
  data: {
    rules: [
      { after: [],               documented: true,  family: 'layout',   position: 1, slug: 'reflow-calls',   title: 'reflow-calls' },
      { after: ['reflow-calls'], documented: false, family: null,       position: 2, slug: 'unwritten-rule', title: 'unwritten-rule' },
      { after: ['unwritten-rule'], documented: true,  family: 'ordering', position: 3, slug: 'group-imports',  title: 'group-imports' }
    ]
  }
}))

const order   = () => mount(PipelineOrder)
const entries = () => order().findAll('.pipeline-order-entry')

const census = (w: ReturnType<typeof order>) => w.findAll('.pipeline-order-census-chip').map(chip => [
  chip.attributes('data-family'), chip.text(), chip.classes('is-later')
])

describe('PipelineOrder', () => {
  it('ends a documented rule\'s entry on its family badge', () => {
    expect(entries()[0].get('use').attributes('href')).toBe('#nib-basket-small')
  })

  it('ends an undocumented rule\'s entry on a plain dot', () => {
    const entry = entries()[1]
    expect(entry.find('use').exists()).toBe(false)
    expect(entry.get('.pipeline-order-glyph').text()).toBe('·')
  })

  it('heads the order with a census of each family\'s passes in the order of its first pass', () => {
    const w = order()
    expect(census(w)).toStrictEqual([['layout', '1', false], ['ordering', '1', false]])
    expect(w.get('.pipeline-order-census-chip').attributes('aria-label')).toBe('Layout, 1 rules')
    expect(w.findAll('.pipeline-order-spine > li')).toHaveLength(3)
  })

  it('marks the pointed rule, the rules it runs after, and the rules that run after it', async () => {
    const w = order()
    const gutter = () => w.findAll('.pipeline-order-gutter').map(cell => [cell.attributes('data-order'), cell.text()])
    expect(gutter()).toStrictEqual([[undefined, ''], [undefined, ''], [undefined, '']])
    await w.findAll('.pipeline-order-link')[1]!.trigger('pointerenter')
    expect(gutter()).toStrictEqual([['earlier', '↑'], ['self', ''], ['later', '↓']])
  })

  it('keeps every pass up to a pointed one at rest and grays every later one', async () => {
    const w = order()
    await w.findAll('.pipeline-order-link')[1]!.trigger('pointerenter')
    expect(w.findAll('.pipeline-order-entry').map(entry => entry.classes().filter(name => name.startsWith('is-'))))
      .toStrictEqual([[], ['is-here'], ['is-later']])
    expect(w.findAll('.pipeline-order-spine > li')[2]!.classes()).toContain('is-later')
    expect(census(w)).toStrictEqual([['layout', '1', false], ['ordering', '0', true]])
    expect(w.get('.pipeline-order-edition').text()).toMatch(/^pass 02 of 3/)
  })

  it('points at a pass from its tick, counts a focused pass, and clears under Escape', async () => {
    const w = order()
    await w.findAll('.pipeline-order-spine > li')[0]!.trigger('pointerenter')
    expect(w.get('.pipeline-order-edition').text()).toMatch(/^pass 01 of 3/)
    const link = w.findAll('.pipeline-order-link')[2]!
    Object.defineProperty(link.element, 'matches', { value: (selector: string) => selector === ':focus-visible' })
    await link.trigger('focusin')
    expect(w.get('.pipeline-order-edition').text()).toMatch(/^pass 03 of 3/)
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }))
    await w.vm.$nextTick()
    expect(w.get('.pipeline-order-edition').text()).toMatch(/^3 rules/)
  })

  it('leaves the pass where a focus draws no ring', async () => {
    const w    = order()
    const link = w.findAll('.pipeline-order-link')[0]!
    Object.defineProperty(link.element, 'matches', { value: () => false })
    await link.trigger('focusin')
    expect(w.get('.pipeline-order-edition').text()).toMatch(/^3 rules/)
  })

  it('marks only a documented rule for the delegated rule card', () => {
    expect(order().findAll('.pipeline-order-link').map(link => link.attributes('data-rule')))
      .toStrictEqual(['reflow-calls', undefined, 'group-imports'])
  })
})
