import { defineLoader } from 'vitepress'

import { getRenderer }          from '../lib/markdown/renderer'
import { SOURCES, stripPrefix } from '../lib/tokens/tokens'
import type { Domain, Token }   from '../lib/tokens/tokens'

declare const data: readonly Token[]
export { data }

export default defineLoader({
  watch: [],
  async load(): Promise<readonly Token[]> {
    const md = await getRenderer()
    return Object.entries(SOURCES).flatMap(([domain, sources]) =>
      sources.map(s => ({
        blurbHtml : md.renderInline(s.blurb),
        domain    : domain as Domain,
        href      : s.href,
        key       : s.key,
        sort      : stripPrefix(s.key)
      })))
  }
})
