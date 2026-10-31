import { runProse } from '../shared/paths'

interface ExitCodeSource {
  code   : number
  detail : readonly string[]
}

export interface ExitCodeEntry extends ExitCodeSource {
  label   : string
  summary : string
}

export const LINT_CODE = 2

export const SOURCES: readonly ExitCodeSource[] = [
  {
    code   : 0,
    detail : [
      'Returned by both `prose check` and `prose format` when the input already conforms, '
      + 'and by `prose format` once its rewrite is written.',
      'A stdin `prose format` that writes the formatted source returns it whatever lint the '
      + 'source holds, since an editor piping a buffer through the command keeps the buffer '
      + 'unformatted on any other code. `prose check -` and the '
      + '[**language server**](/reference/cli#prose-server) still report that lint.',
      '`prose format` also returns it for a rewrite a second run would change, as '
      + '[**Unstable Output**](/reference/cli#unstable-output) sets out.',
      'A CI gate passes with no further work.'
    ]
  },
  {
    code   : 1,
    detail : [
      '`prose check` and `prose format --diff` return this when one or more rules '
      + 'would write an edit, as does a stdin `prose format` under a structured '
      + '`--output-format`, which prints the records and leaves the rewrite unwritten.',
      'Every rewriting rule can produce it.'
    ]
  },
  {
    code   : 2,
    detail : [
      'Returned by both `prose check` and `prose format`, except a stdin `prose format` that '
      + 'writes the formatted source, which returns `0`.'
    ]
  },
  {
    code   : 3,
    detail : [
      'Returned by both subcommands when `ruff_python_parser` fails on the source, or when the '
      + 'source holds syntax the `target-version` does not support or a semantic syntax error such '
      + 'as a `lazy` import inside a function that formatting leaves standing.',
      'No other finding is reported for that file.'
    ]
  },
  {
    code   : 4,
    detail : [
      'Returned for a config-file parse error, a malformed `--select` or `--ignore` '
      + 'flag, an unknown CLI option, or a run naming neither a path nor `--stdin`.',
      'A named path that does not exist returns it too, as does a named file that is not a '
      + '`.py`, `.pyi`, `.pyw`, or `.ipynb` file, and the other named paths still run.',
      'A `prose cache` subcommand returns it on a permission or filesystem failure.',
      'A file the run cannot read, or whose rewrite it cannot write, returns it too, with the '
      + 'error printed to stderr.',
      'A malformed flag stops the whole run, whereas a broken ancestor config fails only '
      + 'the files it governs and the rest proceed.',
      'A stdin run whose `--stdin-filename` names a file under a config that fails to load '
      + 'returns it too, as the path run over that file does.',
      'A rewrite that fails to parse or to compile returns it too, with the file left '
      + 'unwritten.',
      'A rewrite a second run would change returns it too under `prose check --validate`.'
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
