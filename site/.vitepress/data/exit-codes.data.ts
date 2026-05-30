import { defineLoader } from 'vitepress'

import { getRenderer, renderInlineField } from '../lib/markdown/renderer'

interface ExitCode {
  code        : number
  detailHtml  : readonly string[]
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

const SOURCES: readonly ExitCodeSource[] = [
  {
    code   : 0,
    detail : [
      'Returned by both `prose check` and `prose format` when the input is already conforming.',
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
      'The shipped lints contribute: `legacy-union-syntax`, `loose-constants`, '
      + '`no-step-narration`, `single-use-variables`.'
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
      'Pre-empts every other code (the run never reaches the pipeline).'
    ],
    label  : 'Config error',
    summary: 'Config file or argument validation failed.'
  }
]

export default defineLoader({
  watch: [],
  async load(): Promise<readonly ExitCode[]> {
    const md = await getRenderer()
    return renderInlineField(md, SOURCES, 'detail')
  }
})
