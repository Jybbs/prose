import { map } from 'nanostores'

export type FixtureTab = 'after' | 'before'

// One tab entry per fixture id, shared by the toggle island and the pair
// island so either side of the card reflects the other's writes.
export const fixtureTabs = map<Record<string, FixtureTab>>({})

export const setTab = (id: string, tab: FixtureTab): void => fixtureTabs.setKey(id, tab)

export const tabOf = (id: string): FixtureTab => fixtureTabs.get()[id] ?? 'after'

if (import.meta.vitest) {
  const { afterEach, describe, expect, test } = import.meta.vitest

  afterEach(() => fixtureTabs.set({}))

  describe('fixture tab store', () => {
    test('defaults an unseen id to the after tab', () => {
      expect(tabOf('unseen')).toBe('after')
    })

    test('reads back a tab that setTab wrote', () => {
      setTab('card-1', 'before')
      expect(tabOf('card-1')).toBe('before')
      expect(fixtureTabs.get()['card-1']).toBe('before')
    })

    test('keeps a per-id entry so cards stay independent', () => {
      setTab('card-1', 'before')
      setTab('card-2', 'after')
      expect(fixtureTabs.get()).toEqual({ 'card-1': 'before', 'card-2': 'after' })
    })
  })
}
