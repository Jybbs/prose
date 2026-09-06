import { defineLoader } from 'vitepress'

import type { InlineNode }              from '../markdown/inline-nodes'
import { getRenderer, inlineNodeField } from '../markdown/renderer'
import { discoverRuleSlugs }            from '../rules/discovery'
import { rulesDir }                     from '../shared/paths'

interface ExitCode {
  code        : number
  detailNodes : InlineNode[][]
  label       : string
  summary     : string
}

declare const data: readonly ExitCode[]
export { data }

interface ExitCodeSource {
  code    : number
  detail  : readonly string[]
  label   : string
  summary : string
}

const rulesDirectory = rulesDir(import.meta.url)

const SHIPPED_LINTS = discoverRuleSlugs(rulesDirectory)
  .filter(rule => rule.lints)
  .map(rule => `\`${rule.slug}\``)
  .join(', ')

const SOURCES: readonly ExitCodeSource[] = [
  {
    code   : 0,
    detail : [
      'Returned by both `prose check` and `prose format` when the input already conforms.',
      '`prose format` also returns it for a rewrite a second run would change, because that '
      + 'defect belongs to the formatter rather than to the file, and the notice on stderr '
      + 'reports it rather than a code of its own.',
      'A CI gate passes with no further work.'
    ],
    label  : 'Clean',
    summary: 'No findings, no rewrite pending.'
  },
  {
    code   : 1,
    detail : [
      '`prose check` and `prose format --diff` return this when one or more auto-fix rules '
      + 'would write an edit.',
      '`prose format` returns 0 once the rewrite is written.',
      'Every auto-fix rule can produce it.'
    ],
    label  : 'Format would change',
    summary: 'At least one auto-fix rewrite is pending.'
  },
  {
    code   : 2,
    detail : [
      'Returned by both `prose check` and `prose format`.',
      `The shipped lints can produce it: ${SHIPPED_LINTS}.`
    ],
    label  : 'Lint violation',
    summary: 'At least one lint finding was reported.'
  },
  {
    code   : 3,
    detail : [
      'Returned by both subcommands when `ruff_python_parser` fails on the source.',
      'The pipeline does not run, so no other finding is reported.'
    ],
    label  : 'Parse error',
    summary: 'The input could not be parsed as Python.'
  },
  {
    code   : 4,
    detail : [
      'Returned for a config-file parse error, a malformed `--select` or `--ignore` '
      + 'flag, or an unknown CLI option.',
      'A `prose cache` subcommand returns it on a permission or filesystem failure.',
      'A malformed flag stops the whole run, whereas a broken ancestor config fails only '
      + 'the files it governs and the rest proceed.',
      'A rewrite that fails to parse or to compile returns it too, with the file left '
      + 'unwritten.',
      'A rewrite a second run would change returns it too under `prose check --validate`, '
      + 'the opt-in gate for a project that would rather fail CI than read the notice.'
    ],
    label  : 'Config error',
    summary: 'The config file or the arguments failed validation.'
  }
]

export default defineLoader({
  watch: [`${rulesDirectory}/*/*.md`],
  async load(): Promise<readonly ExitCode[]> {
    const md = await getRenderer()
    return inlineNodeField(md, SOURCES, 'detail')
  }
})
