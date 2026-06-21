import { useTabSelect } from '../lib/composables/use-tab-select'

const items = [{ id: 'a', n: 1 }, { id: 'b', n: 2 }, { id: 'c', n: 3 }]

describe('useTabSelect', () => {
  it('starts on the first item', () => {
    const { active, selected } = useTabSelect(items, i => i.id)
    expect(selected.value).toBe('a')
    expect(active.value).toBe(items[0])
  })

  it('tracks the active item as the selection changes', () => {
    const { active, selected } = useTabSelect(items, i => i.id)
    selected.value = 'c'
    expect(active.value).toBe(items[2])
  })

  it('falls back to the first item for an unknown selection', () => {
    const { active, selected } = useTabSelect(items, i => i.id)
    selected.value = 'missing'
    expect(active.value).toBe(items[0])
  })
})
