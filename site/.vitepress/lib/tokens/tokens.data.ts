import { defineLoader } from 'vitepress'

import { exitCodes }                          from '../exit-codes/sources'
import { inlineNodes }                        from '../markdown/inline-nodes'
import { getRenderer }                        from '../markdown/renderer'
import * as help                              from '../reference/cli-help'
import { readPipeline }                       from '../rules/pipeline'
import { proseBinaryPath, repoRoot, siteDir } from '../shared/paths'
import { proseSchema }                        from '../shared/rule-schema'
import * as configKeys                        from './config-keys'
import { destinations }                       from './destinations'
import * as sources                           from './sources'

const root = repoRoot(import.meta.url)
const site = siteDir(import.meta.url)

declare const data: readonly sources.Token[]
export { data }

export default defineLoader({
  watch: [proseBinaryPath(root), `${site}/reference/*.md`],
  async load(): Promise<readonly sources.Token[]> {
    const md            = await getRenderer()
    const schema        = proseSchema(root)
    const destinationOf = destinations(site, configKeys.facetDestinations(schema))
    const pages         = help.readHelp(root)
    const slugs         = new Set(readPipeline(import.meta.url).map(entry => entry.slug))
    const fromHelp      = (domain: sources.HelpDomain): readonly sources.TokenSource[] =>
      sources.HELP_SOURCES[domain].map(s => {
        const line   = help.helpLine(pages, s.key)
        const values = help.listsRuleSet(line, slugs) ? undefined : line.values
        return { ...s, blurb: help.closed(help.withValues(configKeys.firstSentence(line.help), values)) }
      })
    const all: Record<sources.Domain, readonly sources.TokenSource[]> = {
      ...sources.SOURCES,
      'cli-flag'   : fromHelp('cli-flag'),
      'subcommand' : fromHelp('subcommand'),
      'config-key' : configKeys.configKeySources(schema),
      'exit-code'  : exitCodes(root).map(s => ({
        key   : String(s.code),
        href  : '/reference/exit-codes',
        blurb : s.summary
      }))
    }
    return Object.entries(all).flatMap(([domain, domainSources]) =>
      domainSources.map(s => ({
        blurbNodes  : inlineNodes(md, s.blurb),
        destination : destinationOf(s.href),
        domain      : domain as sources.Domain,
        href        : s.href,
        key         : s.key,
        sort        : sources.stripPrefix(s.key)
      })))
  }
})
