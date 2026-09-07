import fs   from 'node:fs'
import os   from 'node:os'
import path from 'node:path'

import { test as base, type MockInstance } from 'vitest'

interface SupportFixtures {
  tmpDir : string
  warn   : MockInstance
}

// Asserts a discovery's slug index and its list agree on membership and order.
export function expectSlugIndex(
  index : (dir: string) => ReadonlyMap<string, unknown>,
  list  : (dir: string) => ReadonlyArray<{ slug: string }>,
  dir   : string
): void {
  expect([...index(dir).keys()]).toStrictEqual(list(dir).map(entry => entry.slug))
}

export const fixtureDir = (metaDir: string, ...parts: string[]): string =>
  path.join(metaDir, 'fixtures', ...parts)

// Supplies a temporary directory removed after the test, and a `console.warn`
// spy the suite's `restoreMocks` default restores.
export const supportTest = base.extend<SupportFixtures>({
  // oxlint-disable-next-line no-empty-pattern -- vitest fixtures require object destructuring
  tmpDir: async ({}, use) => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'prose-test-'))
    await use(dir)
    fs.rmSync(dir, { force: true, recursive: true })
  },
  // oxlint-disable-next-line no-empty-pattern -- vitest fixtures require object destructuring
  warn: async ({}, use) => {
    await use(vi.spyOn(console, 'warn').mockImplementation(() => {}))
  }
})
