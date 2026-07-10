import path from 'node:path'

import { expect, test as base, vi, type MockInstance } from 'vitest'

export function expectMemoized<T>(fn: (dir: string) => T, dir: string): void {
  expect(fn(dir)).toBe(fn(dir))
}

export function expectSlugIndex(
  index : (dir: string) => ReadonlyMap<string, unknown>,
  list  : (dir: string) => ReadonlyArray<{ slug: string }>,
  dir   : string
): void {
  expect([...index(dir).keys()]).toEqual(list(dir).map(entry => entry.slug))
  expectMemoized(index, dir)
}

export const fixtureDir = (metaDir: string, ...parts: string[]): string =>
  path.join(metaDir, 'fixtures', ...parts)

// Fixture supplying a console.warn spy that auto-restores after the test,
// for the with-fallback paths that warn on a swallowed error.
export const warnTest = base.extend<{ warn: MockInstance }>({
  // oxlint-disable-next-line no-empty-pattern -- vitest fixtures require object destructuring
  warn: async ({}, use) => {
    const spy = vi.spyOn(console, 'warn').mockImplementation(() => {})
    await use(spy)
    spy.mockRestore()
  }
})
