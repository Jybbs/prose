import { defineLoader } from 'vitepress'

import { getRenderer, renderInlineHtml } from '../lib/markdown/renderer'
import * as sources                      from '../lib/tokens/sources'

declare const data: readonly sources.Token[]
export { data }

export default defineLoader({
  watch: [],
  async load(): Promise<readonly sources.Token[]> {
    const md = await getRenderer()
    return Object.entries(sources.SOURCES).flatMap(([domain, domainSources]) =>
      domainSources.map(s => ({
        blurbHtml : renderInlineHtml(md, s.blurb),
        domain    : domain as sources.Domain,
        href      : s.href,
        key       : s.key,
        sort      : sources.stripPrefix(s.key)
      })))
  }
})
