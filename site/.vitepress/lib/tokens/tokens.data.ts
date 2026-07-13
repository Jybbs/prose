import { defineLoader } from 'vitepress'

import { getRenderer }                  from '../markdown/renderer'
import { inlineNodes } from '../markdown/inline-nodes'
import * as sources                      from './sources'

declare const data: readonly sources.Token[]
export { data }

export default defineLoader({
  watch: [],
  async load(): Promise<readonly sources.Token[]> {
    const md = await getRenderer()
    return Object.entries(sources.SOURCES).flatMap(([domain, domainSources]) =>
      domainSources.map(s => ({
        blurbNodes : inlineNodes(md, s.blurb),
        domain    : domain as sources.Domain,
        href      : s.href,
        key       : s.key,
        sort      : sources.stripPrefix(s.key)
      })))
  }
})
