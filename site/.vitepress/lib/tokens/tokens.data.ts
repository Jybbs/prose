import { defineLoader } from 'vitepress'

import { exitCodes }                          from '../exit-codes/sources.ts'
import { inlineNodes }                        from '../markdown/inline-nodes.ts'
import { getRenderer }                        from '../markdown/renderer.ts'
import * as help                              from '../reference/cli-help.ts'
import { readPipeline }                       from '../rules/pipeline.ts'
import { pagesDir, proseBinaryPath, repoRoot } from '../shared/paths.ts'
import { proseSchema }                        from '../shared/rule-schema.ts'
import * as configKeys                        from './config-keys.ts'
import { destinations }                       from './destinations.ts'
import * as sources                           from './sources.ts'

const root   = repoRoot(import.meta.url)
const srcDir = pagesDir(import.meta.url)

declare const data: readonly sources.Token[]
export { data }

export default defineLoader({
  watch: [proseBinaryPath(root), `${srcDir}/reference/*.md`],
  async load(): Promise<readonly sources.Token[]> {
    const md            = await getRenderer()
    const schema        = proseSchema(root)
    const destinationOf = destinations(srcDir, configKeys.facetDestinations(schema))
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
