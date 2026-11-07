import { defineLoader } from 'vitepress'

import { inlineNodes }       from '../markdown/inline-nodes'
import type { InlineParser } from '../markdown/inline-nodes'
import { getRenderer }       from '../markdown/renderer'
import { readPipeline }      from '../rules/pipeline'
import type { ConfigRow }    from '../shared/config-row'
import * as paths            from '../shared/paths'
import * as help             from './cli-help'

export type FlagTable = Exclude<help.HelpCommand, '' | 'cache'> | 'global'

export type CliFlags = Record<FlagTable, readonly ConfigRow[]>

const BUILTINS = new Set(['--help', '--version'])

const root = paths.repoRoot(import.meta.url)

declare const data: CliFlags
export { data }

// Builds the table row one help entry reads as, naming a value list equal to
// the registered rule set as rule slugs rather than spelling out every slug.
export function flagRow(md: InlineParser, entry: help.HelpEntry, slugs: ReadonlySet<string>): ConfigRow {
  const ruleSet = entry.values?.length === slugs.size && entry.values.every(value => slugs.has(value))
  const type    = ruleSet                    ? '[rule slugs](/rules/)'
                : entry.values !== undefined ? entry.values.map(value => `\`${value}\``).join(' \\| ')
                : entry.value !== undefined  ? `\`<${entry.value}>\``
                : entry.name.startsWith('-') ? 'bool'
                : 'path'
  return {
    default      : entry.default ?? (entry.name.startsWith('-') && entry.value === undefined ? 'off' : 'unset'),
    key          : entry.short === undefined ? entry.name : `${entry.name}, -${entry.short}`,
    meaningNodes : inlineNodes(md, help.closed(entry.help)),
    typeNodes    : inlineNodes(md, type)
  }
}

export default defineLoader({
  watch : [paths.proseBinaryPath(root)],
  async load(): Promise<CliFlags> {
    const md     = await getRenderer()
    const pages  = help.readHelp(root)
    const slugs  = new Set(readPipeline(import.meta.url).map(entry => entry.slug))
    const global = new Set(pages[''].options.map(entry => entry.name))

    const rows = (entries: readonly help.HelpEntry[]): readonly ConfigRow[] =>
      entries.filter(entry => !BUILTINS.has(entry.name)).map(entry => flagRow(md, entry, slugs))
    const own = (command: help.HelpCommand): readonly ConfigRow[] =>
      rows([...pages[command].arguments, ...pages[command].options.filter(entry => !global.has(entry.name))])

    return {
      check       : own('check'),
      completions : own('completions'),
      format      : own('format'),
      global      : rows(pages[''].options),
      rules       : own('rules'),
      server      : own('server')
    }
  }
})
