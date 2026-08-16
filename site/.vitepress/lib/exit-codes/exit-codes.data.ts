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
      'Returned by both `prose check` and `prose format` when the input is already conforming.',
      '`prose format` also returns this once a rewrite lands, including one a second run '
      + 'would change again, because that defect belongs to the formatter rather than to '
      + 'the file and the notice on stderr is what surfaces it.',
      'CI gates pass without further work.'
    ],
    label  : 'Clean',
    summary: 'No diagnostics, no rewrites pending.'
  },
  {
    code   : 1,
    detail : [
      '`prose check` returns this when one or more auto-fix rules would emit edits.',
      '`prose format` returns 0 once the rewrite lands.',
      'Every auto-fix rule contributes here.'
    ],
    label  : 'Format would change',
    summary: 'At least one auto-fix diagnostic is pending.'
  },
  {
    code   : 2,
    detail : [
      'Surfaces under both `prose check` and `prose format`.',
      `The shipped lints contribute: ${SHIPPED_LINTS}.`
    ],
    label  : 'Lint violation',
    summary: 'At least one lint-only diagnostic surfaced.'
  },
  {
    code   : 3,
    detail : [
      'Surfaces under both subcommands when `ruff_python_parser` fails on the source.',
      'The pipeline does not run, leaving no other diagnostics to fire.'
    ],
    label  : 'Parse error',
    summary: 'Input could not be parsed as Python.'
  },
  {
    code   : 4,
    detail : [
      'Surfaces from config-file parse errors, malformed `--select` / '
      + '`--ignore` flags, or unknown CLI options.',
      'A malformed flag pre-empts the whole run, whereas a broken ancestor '
      + 'config fails only the files it governs while the rest proceed.',
      'A rewrite that fails to re-parse or to compile lands here too, its '
      + 'file left unwritten.',
      '`prose check --validate` adds a rewrite a second run would change, the opt-in gate '
      + 'for a project that would rather fail CI than read the notice.'
    ],
    label  : 'Config error',
    summary: 'Config file or argument validation failed.'
  }
]

export default defineLoader({
  watch: [`${rulesDirectory}/*/*.md`],
  async load(): Promise<readonly ExitCode[]> {
    const md = await getRenderer()
    return inlineNodeField(md, SOURCES, 'detail')
  }
})
