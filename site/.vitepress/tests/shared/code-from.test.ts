import fs from 'node:fs'

import { Window } from 'happy-dom'

import * as render      from '../../lib/fixtures/render'
import * as walker      from '../../lib/fixtures/walker'
import { codeFrom }     from '../../lib/shared/code-from'
import { CASES, CRATE } from '../corpus'

const OVERRIDING = CASES.find(entry => walker.readOverrideToml(entry.inputPath))!

// Renders through Shiki in Node, whose `Response` the happy-dom environment
// replaces, and parses with a happy-dom window's `DOMParser`.
describe('codeFrom', () => {
  beforeEach(() => { vi.stubGlobal('DOMParser', new Window().DOMParser) })

  it('recovers the input and the override a case renders, short of the closing newline', async () => {
    const entry = (await render.fixtureEntries(CRATE, [OVERRIDING.id]))[OVERRIDING.id]
    expect(`${codeFrom(entry.inputHtml)}\n`).toBe(fs.readFileSync(OVERRIDING.inputPath, 'utf8'))
    expect(`${codeFrom(entry.configHtml!)}\n`).toBe(walker.readOverrideToml(OVERRIDING.inputPath))
  })

  it('leaves the lang chip and copy button out of the text', () => {
    expect(codeFrom('<div><button class="copy"></button><span class="lang">python</span><pre><code>a = 1\n</code></pre></div>'))
      .toBe('a = 1')
  })
})
