import * as help from '../../lib/reference/cli-help'
import { HELP }   from '../schema'

const ONE_LINE = `Rewrite files

Usage: prose format [OPTIONS] [PATH]...

Arguments:
  [PATH]...  Files to format, or \`-\` to read stdin. Omit when using \`--stdin\`

Options:
      --color <WHEN>                   When to use colored output [default: auto] [possible values: auto, always, never]
  -q, --quiet                          Reduce the summary to a bare count line
      --diff                           Show a unified diff
`

const LONG = `Run the language server

Usage: prose server [OPTIONS]

Commands:
  clean    Clear every cached entry
  info     Print the cache directory

Options:
      --transport <TRANSPORT>
          Transport the server speaks over. Only stdio is supported

          Possible values:
          - stdio: Communicate over stdin and stdout

          [default: stdio]

      --verbose
          Print extra diagnostic information
`

describe('parseHelp', () => {
  it('reads the one-line layout, lifting the default and the possible values', () => {
    const page = help.parseHelp(ONE_LINE)
    expect(page.arguments).toStrictEqual([
      { help: 'Files to format, or `-` to read stdin. Omit when using `--stdin`', name: 'PATH...' }
    ])
    expect(page.options).toStrictEqual([
      { default: 'auto', help: 'When to use colored output', name: '--color', value: 'WHEN', values: ['auto', 'always', 'never'] },
      { help: 'Reduce the summary to a bare count line', name: '--quiet', short: 'q' },
      { help: 'Show a unified diff', name: '--diff' }
    ])
  })

  it('reads the layout setting help below its flag, with a listed value set', () => {
    const page = help.parseHelp(LONG)
    expect(page.options).toStrictEqual([
      { default: 'stdio', help: 'Transport the server speaks over. Only stdio is supported', name: '--transport', value: 'TRANSPORT', values: ['stdio'] },
      { help: 'Print extra diagnostic information', name: '--verbose' }
    ])
    expect([...page.commands]).toStrictEqual([['clean', 'Clear every cached entry'], ['info', 'Print the cache directory']])
  })
})

describe('closed', () => {
  it.each([
    ['Show a unified diff', 'Show a unified diff.'],
    ['Read stdin.',         'Read stdin.']
  ])('closes %j as %j', (text, sentence) => {
    expect(help.closed(text)).toBe(sentence)
  })
})

describe('helpLine', () => {
  it('reads a subcommand line off its parent help page', () => {
    expect(helpLine('prose cache info')).toMatch(/^Print the cache directory/)
    expect(helpLine('prose check')).toMatch(/exiting non-zero on either$/)
  })

  it('reads a flag by the name its key opens on', () => {
    expect(helpLine('--select <slug>')).toMatch(/^Comma-separated rule slugs to run/)
  })

  it('carries the values clap lists for a flag', () => {
    expect(help.helpLine(HELP, '--output-format').values).toStrictEqual(['github', 'json', 'sarif', 'text'])
  })

  it.each(['prose cache vacuum', 'prose nothing', '--no-such-flag'])('throws for %j, which no page prints', key => {
    expect(() => helpLine(key)).toThrow(/prints (no )?a? ?line/)
  })
})

describe('withValues', () => {
  it.each([
    ['Output format for diagnostics',  ['text', 'json'], 'Output format for diagnostics, `text` or `json`'],
    ['Pick one.',                      ['a', 'b', 'c'],  'Pick one, `a`, `b`, or `c`'],
    ['Read stdin.',                    undefined,        'Read stdin.']
  ])('appends to %j the values %j', (sentence, values, expected) => {
    expect(help.withValues(sentence, values)).toBe(expected)
  })
})

function helpLine(key: string): string {
  return help.helpLine(HELP, key).help
}
