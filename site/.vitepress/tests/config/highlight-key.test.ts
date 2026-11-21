import fs   from 'node:fs'
import path from 'node:path'

import { HighlightCache } from '../../lib/markdown/highlight-cache'
import { siteDir }        from '../../lib/shared/paths'

import type { readPublishedVersion } from '../../lib/shared/published-version'

vi.mock('../../lib/shared/published-version', () => ({
  readPublishedVersion: vi.fn<typeof readPublishedVersion>().mockResolvedValue('9.8.7')
}))

const HIGHLIGHTER_OPTIONS = [
  'codeTransformers', 'colorReplacements', 'defaultHighlightLang', 'highlight', 'languageAlias',
  'languages', 'shikiSetup', 'theme'
] as const

const read                = vi.spyOn(HighlightCache, 'read')
const { default: config } = await import('../../config')
const [, inputs]          = read.mock.calls[0]!

describe('the highlight cache key', () => {
  it.each(HIGHLIGHTER_OPTIONS)('takes the %s option the highlighter reads', option => {
    expect(inputs.options[option]).toBe(config.markdown?.[option])
  })

  it('takes the text of the lockfile', () => {
    expect(inputs.lockfile).toBe(fs.readFileSync(path.join(siteDir(import.meta.url), 'bun.lock'), 'utf8'))
  })
})
