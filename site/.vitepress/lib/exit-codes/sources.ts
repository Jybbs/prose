import { runProse } from '../shared/paths'

export const OUTCOMES = ['success', 'findings', 'error'] as const

export type Outcome = (typeof OUTCOMES)[number]

// Names the band each outcome heads in the index.
export const OUTCOME_LABELS: Record<Outcome, string> = {
  error    : 'Errors',
  findings : 'Findings',
  success  : 'Passes'
}

// Names the facts each code's detail lists, in the order `facts` holds them.
export const FACT_LABELS = ['Returned by', 'CI gate', 'Next step', 'Note'] as const

interface ExitCodeSource {
  code    : number
  facts   : readonly [returned: string, gate: string, next: string, note: string]
  outcome : Outcome
}

export interface ExitCodeEntry extends ExitCodeSource {
  label   : string
  summary : string
}

export const LINT_CODE = 2

export const SOURCES: readonly ExitCodeSource[] = [
  {
    code    : 0,
    outcome : 'success',
    facts   : [
      'Both `prose check` and `prose format` when the input already conforms, and `prose format` '
      + 'once its rewrite is written. A stdin `prose format` that writes the formatted source '
      + 'returns it whatever lint the source holds, since an editor such as Helix keeps a buffer '
      + 'it pipes through the command unformatted on any other code.',
      'Passes with no further work.',
      'Nothing to do.',
      '`prose format` also returns it for a rewrite a second run would change, as '
      + '[**Unstable Output**](/reference/cli#unstable-output) sets out, and `prose check -` and '
      + 'the [**language server**](/reference/cli#prose-server) still report the lint a stdin '
      + '`prose format` leaves.'
    ]
  },
  {
    code    : 1,
    outcome : 'findings',
    facts   : [
      '`prose check` and `prose format --diff` when one or more rules would write an edit, as '
      + 'does a stdin `prose format` under a structured `--output-format`, which prints the '
      + 'records and leaves the rewrite unwritten.',
      'Fails.',
      'Run `prose format` and commit the rewrite.',
      'Every rewriting rule can produce it.'
    ]
  },
  {
    code    : 2,
    outcome : 'findings',
    facts   : [
      'Both `prose check` and `prose format`.',
      'Fails.',
      'Fix each finding, or silence it with `# prose: ignore[<rule>]`.',
      'The shipped lints can produce it:'
    ]
  },
  {
    code    : 3,
    outcome : 'error',
    facts   : [
      'Both subcommands when `ruff_python_parser` fails on the source, or when the source holds '
      + 'syntax the `target-version` does not support or a semantic syntax error such as a `lazy` '
      + 'import inside a function that formatting leaves standing.',
      'Fails.',
      'Fix the syntax, or raise `target-version` to the Python the source uses.',
      'No other finding is reported for that file.'
    ]
  },
  {
    code    : 4,
    outcome : 'error',
    facts   : [
      'A config-file parse error, a malformed `--select` or `--ignore` flag, an unknown CLI '
      + 'option, or a run naming neither a path nor `--stdin`. A named path that does not exist '
      + 'returns it too, as do a named file that is not a `.py`, `.pyi`, `.pyw`, or `.ipynb` file, '
      + 'a file the run cannot read or whose rewrite it cannot write, a `prose cache` subcommand\'s '
      + 'permission or filesystem failure, a stdin run whose `--stdin-filename` names a file under '
      + 'a config that fails to load, a rewrite that fails to parse or to compile, and under '
      + '`prose check --validate` a rewrite a second run would change.',
      'Fails.',
      'Correct the config file, the flag, or the file the error on stderr names.',
      'A malformed flag stops the whole run, whereas a broken ancestor config fails only the '
      + 'files it governs, a missing or unsupported named path leaves the other named paths to '
      + 'run, and a rewrite that fails leaves its file unwritten.'
    ]
  }
]

// Pairs each source with the label and summary the `prose --help` matrix
// prints for its code, in the `<code>    <Label>: <summary>` row below `Exit codes:`.
export function exitCodesFrom(help: string): readonly ExitCodeEntry[] {
  const matrix = help.slice(help.indexOf('Exit codes:'))
  const rows   = new Map([...matrix.matchAll(/^ {2}(\d+) +([^:\n]+): (.+)$/gm)]
    .map(([, code, label, summary]) => [Number(code), { label, summary }] as const))
  return SOURCES.map(source => {
    const row = rows.get(source.code)
    if (!row) throw new Error(`prose --help prints no exit-code row for ${source.code}`)
    return {
      ...source,
      label   : row.label,
      summary : `${row.summary[0].toUpperCase()}${row.summary.slice(1)}.`
    }
  })
}

// Reads the exit codes the binary under `root` documents from its `--help`.
export function exitCodes(root: string): readonly ExitCodeEntry[] {
  return exitCodesFrom(runProse(root, ['--help']))
}
