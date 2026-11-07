import * as cacache              from 'cacache'
import type { MarkdownRenderer } from 'vitepress'

import { HighlightCache } from '../../lib/markdown/highlight-cache'
import { supportTest }    from '../support'

type Highlight = (code: string, lang: string, attrs: string) => string

const INPUTS = { colorReplacements: {}, data: {}, theme: 'github-dark', transformers: [] }

// Builds a renderer stub whose highlighter files the block's HTML as a pending
// placeholder, the way the async renderer does, wrapped in the cache `dir`
// holds under `inputs`.
async function cached(dir: string, inputs = INPUTS) {
  const placeholderMap = new Map<string, [Promise<string>, string, string, string]>()
  const inner          = vi.fn<Highlight>((code, lang, attrs) => {
    placeholderMap.set(`id-${placeholderMap.size}`, [Promise.resolve(`<pre>${code}</pre>`), code, lang, attrs])
    return '<pre><!--placeholder--></pre>'
  })
  const md    = { options: { highlight: inner }, placeholderMap } as unknown as MarkdownRenderer
  const cache = await HighlightCache.read(dir, inputs)
  cache.plugin(md)
  return { cache, highlight: md.options.highlight as Highlight, inner }
}

describe('HighlightCache', () => {
  supportTest('serves a block a previous build highlighted', async ({ tmpDir }) => {
    const first = await cached(tmpDir)
    expect(first.highlight('x = 1', 'python', '')).toBe('<pre><!--placeholder--></pre>')
    await first.cache.flush()
    const second = await cached(tmpDir)
    expect(second.highlight('x = 1', 'python', '')).toBe('<pre>x = 1</pre>')
    expect(second.inner).not.toHaveBeenCalled()
  })

  supportTest('stores a highlighter result that returns at once', async ({ tmpDir }) => {
    const md    = { options: { highlight: (code: string) => `<pre>${code}</pre>` }, placeholderMap: new Map() }
    const first = await HighlightCache.read(tmpDir, INPUTS)
    first.plugin(md as unknown as MarkdownRenderer)
    ;(md.options.highlight as Highlight)('y', 'python', '')
    await first.flush()
    expect(Object.keys(await cacache.ls(tmpDir))).toHaveLength(1)
  })

  supportTest('highlights again once a block or a shared input changes', async ({ tmpDir }) => {
    const first = await cached(tmpDir)
    first.highlight('x = 1', 'python', '')
    await first.cache.flush()
    const moved = await cached(tmpDir, { ...INPUTS, data: { rule: ['finding'] } })
    moved.highlight('x = 1', 'python', '')
    moved.highlight('x = 1', 'python', 'lint=rule')
    expect(moved.inner).toHaveBeenCalledTimes(2)
  })

  supportTest('prunes each entry the build did not reach', async ({ tmpDir }) => {
    const first = await cached(tmpDir)
    first.highlight('stale', 'python', '')
    await first.cache.flush()
    const second = await cached(tmpDir)
    second.highlight('live', 'python', '')
    await second.cache.flush()
    expect(Object.keys(await cacache.ls(tmpDir))).toHaveLength(1)
  })
})
