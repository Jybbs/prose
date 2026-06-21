import path from 'node:path'

import { test as base, vi, type MockInstance } from 'vitest'

export const fixtureDir = (...parts: string[]): string =>
  path.join(import.meta.dirname, 'fixtures', ...parts)

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
