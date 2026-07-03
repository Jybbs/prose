import { map } from 'nanostores'

export type FixtureTab = 'after' | 'before'

// One tab entry per fixture id, shared by the toggle island and the pair
// island so either side of the card reflects the other's writes.
export const fixtureTabs = map<Record<string, FixtureTab>>({})

export const setTab = (id: string, tab: FixtureTab): void => fixtureTabs.setKey(id, tab)

export const tabOf = (id: string): FixtureTab => fixtureTabs.get()[id] ?? 'after'
