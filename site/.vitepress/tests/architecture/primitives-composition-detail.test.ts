// @vitest-environment happy-dom
import { mount } from '@vue/test-utils'

import { PrimitivesCompositionDetail } from '../../theme/components/architecture/PrimitivesCompositionDetail'

vi.mock('../../lib/rules/rules.data', async () =>
  (await import('../rules-data-stub')).rulesDataStub([{ family: 'layout', slug: 'reflow-calls' }]))

vi.mock('../../lib/architecture/primitives-composition.data', () => ({
  data: {
    bySlug: {
      source: {
        consumedBy   : ['reflow-calls'],
        consumes     : [],
        layer        : 'base',
        name         : 'Source',
        slug         : 'source',
        summaryNodes : []
      }
    }
  }
}))

const card = () => mount(PrimitivesCompositionDetail, {
  props  : { focused: 'source' }
})

describe('PrimitivesCompositionDetail', () => {
  it('leads each rule the focused primitive feeds with its family badge', () => {
    expect(card().get('.chip use').attributes('href')).toBe('#nib-basket-small')
  })

  it('marks each rule the focused primitive feeds for the delegated rule card', () => {
    expect(card().get('.chip').attributes('data-rule')).toBeTruthy()
  })
})
