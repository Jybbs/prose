import { defineLoader } from 'vitepress'

import { SOURCES as EXIT_CODES }     from '../exit-codes/sources'
import { inlineNodes }               from '../markdown/inline-nodes'
import { getRenderer }               from '../markdown/renderer'
import { proseBinaryPath, repoRoot } from '../shared/paths'
import { proseSchema }               from '../shared/rule-schema'
import { configKeySources }          from './config-keys'
import * as sources                  from './sources'

const root = repoRoot(import.meta.url)

declare const data: readonly sources.Token[]
export { data }

export default defineLoader({
  watch: [proseBinaryPath(root)],
  async load(): Promise<readonly sources.Token[]> {
    const md  = await getRenderer()
    const all: Record<sources.Domain, readonly sources.TokenSource[]> = {
      ...sources.SOURCES,
      'config-key' : configKeySources(proseSchema(root)),
      'exit-code'  : EXIT_CODES.map(s => ({
        key   : String(s.code),
        href  : '/reference/exit-codes',
        blurb : s.summary
      }))
    }
    return Object.entries(all).flatMap(([domain, domainSources]) =>
      domainSources.map(s => ({
        blurbNodes : inlineNodes(md, s.blurb),
        domain     : domain as sources.Domain,
        href       : s.href,
        key        : s.key,
        sort       : sources.stripPrefix(s.key)
      })))
  }
})
