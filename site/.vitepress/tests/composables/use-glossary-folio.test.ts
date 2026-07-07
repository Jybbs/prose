vi.mock('../../lib/glossary/glossary.data', () => ({
  data: {
    entries: {
      apple:  { aliases: ['fruit'], initial: 'A', slug: 'apple'  },
      banana: { aliases: [],        initial: 'B', slug: 'banana' },
      cherry: { aliases: [],        initial: 'C', slug: 'cherry' }
    }
  }
}))

import { useGlossaryFolio } from '../../lib/composables/use-glossary-folio'

const folio = useGlossaryFolio()

beforeEach(() => {
  folio.query.value    = ''
  folio.selected.value = folio.ordered[0]!.slug
})

describe('useGlossaryFolio', () => {
  it('orders every entry and starts on the first', () => {
    expect(folio.ordered.map(e => e.slug)).toEqual(['apple', 'banana', 'cherry'])
    expect(folio.active.value?.slug).toBe('apple')
    expect(folio.activeIndex.value).toBe(0)
  })

  it('shows all entries for an empty query and narrows on a slug or alias match', () => {
    expect(folio.filtered.value).toHaveLength(3)
    folio.query.value = 'fruit'
    expect(folio.filtered.value.map(e => e.slug)).toEqual(['apple'])
  })

  it('groups the filtered entries by initial', () => {
    expect(folio.grouped.value.map(([initial]) => initial)).toEqual(['A', 'B', 'C'])
    expect(folio.grouped.value[0]).toHaveLength(2)
  })

  it('steps the selection forward and wraps back through the pool', () => {
    folio.step(1)
    expect(folio.selected.value).toBe('banana')
    folio.step(-1)
    expect(folio.selected.value).toBe('apple')
    expect(folio.activeIndex.value).toBe(0)
  })
})
