import { hash }                  from 'ohash'
import type { MarkdownRenderer } from 'vitepress'

import * as cache           from '../shared/cache'
import type { RenderGraph } from '../shared/render-graph'

type Highlight = (code: string, lang: string, attrs: string) => string

// Caches the HTML the renderer's fence highlighter writes for each block under
// `dir`, keyed on the block's code, language, and fence attributes beside the
// inputs every block shares. Every entry loads before the first render, since
// the renderer calls its highlighter synchronously.
export class HighlightCache {
  private readonly keys   = new Set<string>()
  private readonly writes = [] as Promise<void>[]

  private constructor(
    private readonly dir     : string,
    private readonly base    : string,
    private readonly entries : Map<string, string>
  ) {}

  // Keys the cache on the themes and color replacements, `graph`, which holds the
  // source of every module the renderer's transformers reach and the pinned
  // version of every package they import, shiki and VitePress among them, and
  // `data`, which holds whatever a transformer reads beyond those modules.
  static async read(dir: string, inputs: {
    colorReplacements : unknown
    data              : unknown
    graph             : RenderGraph
    theme             : unknown
  }): Promise<HighlightCache> {
    const entries = new Map([...await cache.readEntries(dir)].map(([key, data]) => [key, data.toString('utf8')]))
    return new HighlightCache(dir, hash(inputs), entries)
  }

  // Wraps the fence highlighter of `md` in the cache. The renderer's highlighter
  // returns a placeholder and files the pending HTML under the newest key of
  // `md.placeholderMap`, which the wrapper stores once it resolves. A rejection
  // stores nothing, since the renderer awaits the same promise and reports it.
  plugin = (md: MarkdownRenderer): void => {
    const highlight = md.options.highlight as Highlight
    const cached: Highlight = (code, lang, attrs) => {
      const key = hash({ attrs, base: this.base, code, lang })
      this.keys.add(key)
      const hit = this.entries.get(key)
      if (hit !== undefined) return hit
      const pending = md.placeholderMap.size
      const html    = highlight(code, lang, attrs)
      const result  = md.placeholderMap.size > pending ? [...md.placeholderMap.values()].at(-1)![0] : html
      this.writes.push(Promise.resolve(result).then(
        written => this.store(key, written),
        () => { this.keys.delete(key) }
      ))
      return html
    }
    md.options.highlight = cached
  }

  // Writes every pending entry and drops each entry the build did not reach.
  async flush(): Promise<void> {
    await Promise.all(this.writes)
    await cache.pruneEntries(this.dir, this.keys)
  }

  private async store(key: string, html: string): Promise<void> {
    this.entries.set(key, html)
    await cache.writeEntry(this.dir, key, html)
  }
}
