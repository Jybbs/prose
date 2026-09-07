import fs   from 'node:fs'
import os   from 'node:os'
import path from 'node:path'

import { test as base, vi, type MockInstance } from 'vitest'

interface SupportFixtures {
  tmpDir : string
  warn   : MockInstance
}

export const fixtureDir = (metaDir: string, ...parts: string[]): string =>
  path.join(metaDir, 'fixtures', ...parts)

// Supplies a temporary directory and a `console.warn` spy, removing the
// directory and restoring the spy after the test.
export const supportTest = base.extend<SupportFixtures>({
  // oxlint-disable-next-line no-empty-pattern -- vitest fixtures require object destructuring
  tmpDir: async ({}, use) => {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'prose-test-'))
    await use(dir)
    fs.rmSync(dir, { force: true, recursive: true })
  },
  // oxlint-disable-next-line no-empty-pattern -- vitest fixtures require object destructuring
  warn: async ({}, use) => {
    const spy = vi.spyOn(console, 'warn').mockImplementation(() => {})
    await use(spy)
    spy.mockRestore()
  }
})
