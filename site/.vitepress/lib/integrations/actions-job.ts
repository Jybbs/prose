import { PYPI_PACKAGE } from '../shared/constants'

// The output format `prose check` writes when no `--output-format` names one.
export const DEFAULT_FORMAT = 'text'

// The output format a later step uploads, which the job writes to a file.
export const UPLOADED_FORMAT = 'sarif'

const CACHE = `      - uses: actions/cache@v6
        with:
          path: ~/.cache/prose
          key: prose-\${{ runner.os }}-\${{ hashFiles('prose.toml', '.config/prose.toml', 'pyproject.toml') }}`

const HEAD = `name: prose

on:
  pull_request:
  push:
    branches: [main]

jobs:
  check:
    runs-on: ubuntu-latest`

const PERMISSIONS = `    permissions:
      actions: read
      contents: read
      security-events: write`

const SETUP = `      - uses: actions/checkout@v7
      - uses: astral-sh/setup-uv@v10.3.3
        with:
          enable-cache: true`

const UPLOAD = `      - uses: github/codeql-action/upload-sarif@v4
        if: \${{ !cancelled() }}
        with:
          sarif_file: prose.sarif`

// One choice of the job the GitHub Actions page builds, naming the output format
// `prose check` writes and the steps added beside it.
export interface JobChoice {
  cache  : boolean
  format : string
  ruff   : boolean
}

// Reads the `prose check` command one choice runs, the format flag left out
// where the choice takes the default.
export function checkCommand(format: string): string {
  const flag = format === DEFAULT_FORMAT ? '' : ` --output-format ${format}`
  const sink = format === UPLOADED_FORMAT ? ` > prose.${format}` : ''
  return `prose check${flag} .${sink}`
}

// Keys a choice so the build renders each workflow once and the page reads it
// back by the same key.
export function choiceKey({ cache, format, ruff }: JobChoice): string {
  return [format, cache ? 'cache' : '', ruff ? 'ruff' : ''].filter(Boolean).join('+')
}

// Lists every choice the head row reaches over the formats `prose check` takes.
export function everyChoice(formats: readonly string[]): JobChoice[] {
  return formats.flatMap(format => [false, true].flatMap(cache =>
    [false, true].map(ruff => ({ cache, format, ruff }))))
}

// Writes the workflow one choice builds, the cache step restoring the cache
// ahead of the run and the Ruff steps installing and running Ruff's linter
// ahead of `prose check`.
export function jobYaml(choice: JobChoice): string {
  const uploads = choice.format === UPLOADED_FORMAT
  return [
    HEAD,
    ...(uploads ? [PERMISSIONS] : []),
    '    steps:',
    SETUP,
    ...(choice.cache ? [CACHE] : []),
    ...(choice.ruff ? ['      - run: uv tool install ruff'] : []),
    `      - run: uv tool install ${PYPI_PACKAGE}`,
    ...(choice.ruff ? ['      - run: ruff check .'] : []),
    `      - run: ${checkCommand(choice.format)}`,
    ...(uploads ? [UPLOAD] : [])
  ].join('\n')
}
