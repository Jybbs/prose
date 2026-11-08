import { runProse }    from '../shared/paths'
import { stripSuffix } from '../shared/strip-suffix'

// The subcommands whose `--help` the site reads, keyed by the words after
// `prose`, the empty key naming the top-level help.
const HELP_COMMANDS = ['', 'cache', 'check', 'completions', 'format', 'rules', 'server'] as const

export type HelpCommand = (typeof HELP_COMMANDS)[number]

// One argument or option row a `--help` page prints, with clap's trailing
// `[default: …]` and `[possible values: …]` brackets lifted out of `help`.
export interface HelpEntry {
  default? : string
  help     : string
  name     : string
  short?   : string
  value?   : string
  values?  : readonly string[]
}

export interface HelpPage {
  arguments : readonly HelpEntry[]
  commands  : ReadonlyMap<string, string>
  options   : readonly HelpEntry[]
}

const BRACKET = /\s*\[(default|possible values): ([^\]]*)\]/g
const LISTED  = /\s*Possible values:/
const ENTRY   = /^ {2}(?:-(\w), )?\s*(--[\w-]+|\[?[A-Z_]+\]?(?:\.\.\.)?|<[A-Z_]+>)(?: <([A-Z_]+)>)?(?: {2,}(.*))?$/
const SECTION = /^([A-Z][\w ]*):$/

const VALUE_LIST = new Intl.ListFormat('en', { type: 'disjunction' })

// Lifts the bracketed default and possible values out of one help text, and
// the `Possible values:` list clap prints in place of the bracket where each
// value carries its own help.
function entryFrom(name: string, short: string | undefined, value: string | undefined, text: string): HelpEntry {
  const [prose, listed] = text.split(LISTED)
  const brackets        = Object.fromEntries([...text.matchAll(BRACKET)].map(([, kind, body]) => [kind, body]))
  const values          = listed === undefined
    ? brackets['possible values']?.split(', ')
    : [...listed.matchAll(/- ([\w-]+):/g)].map(([, listedValue]) => listedValue)
  return {
    help : prose.replaceAll(BRACKET, '').replaceAll(/\s+/g, ' ').trim(),
    name : name.replaceAll(/[[\]<>]/g, ''),
    ...(short === undefined ? {} : { short }),
    ...(value === undefined ? {} : { value }),
    ...(brackets.default === undefined ? {} : { default: brackets.default }),
    ...(values === undefined ? {} : { values })
  }
}

// Parses one clap `--help` page, reading both the one-line layout and the
// layout that sets each help text on the indented lines below its flag.
export function parseHelp(text: string): HelpPage {
  const sections = new Map<string, HelpEntry[]>()
  const commands = new Map<string, string>()
  let section    = ''
  let pending: { name: string, short?: string, value?: string, lines: string[] } | undefined

  const flush = (): void => {
    if (pending === undefined) return
    const list = sections.get(section) ?? []
    list.push(entryFrom(pending.name, pending.short, pending.value, pending.lines.join(' ')))
    sections.set(section, list)
    pending = undefined
  }

  for (const line of text.split('\n')) {
    const heading = SECTION.exec(line)
    if (heading) {
      flush()
      section = heading[1]
      continue
    }
    if (section === 'Commands') {
      const command = /^ {2}([\w-]+) {2,}(.+)$/.exec(line)
      if (command) commands.set(command[1], command[2])
      continue
    }
    if (section !== 'Options' && section !== 'Arguments') continue
    const entry = ENTRY.exec(line)
    if (entry) {
      flush()
      const [, short, name, value, rest] = entry
      pending = { lines: rest === undefined ? [] : [rest], name, short, value }
    } else if (pending !== undefined && line.trim() !== '') {
      pending.lines.push(line.trim())
    }
  }
  flush()
  return { arguments: sections.get('Arguments') ?? [], commands, options: sections.get('Options') ?? [] }
}

// Reads the `--help` page of every command the site documents.
export function readHelp(root: string): Record<HelpCommand, HelpPage> {
  return Object.fromEntries(HELP_COMMANDS.map(command => [
    command,
    parseHelp(runProse(root, [...command.split(' ').filter(Boolean), '--help']))
  ])) as Record<HelpCommand, HelpPage>
}

// Returns a help text closed with the period clap drops from a single-paragraph
// doc comment.
export function closed(help: string): string {
  return help.endsWith('.') ? help : `${help}.`
}

// Returns the help text `prose <words>` prints for the subcommand the last
// word names, or for the flag a key such as `--select <slug>` opens on beside
// the values clap lists for it, throwing where no help page prints a line for it.
export function helpLine(
  pages : Record<HelpCommand, HelpPage>,
  key   : string
): Pick<HelpEntry, 'help' | 'values'> {
  const words = key.split(' ')
  if (words[0] === 'prose') {
    const parent = words.slice(1, -1).join(' ') as HelpCommand
    const help   = pages[parent]?.commands.get(words.at(-1)!)
    if (help === undefined) throw new Error(`prose ${parent} --help prints no line for \`${key}\``)
    return { help }
  }
  const option = HELP_COMMANDS.flatMap(command => pages[command].options).find(entry => entry.name === words[0])
  if (option === undefined) throw new Error(`no prose --help page prints a line for \`${words[0]}\``)
  return option
}

// Reports whether an entry's values are every registered rule slug, which a
// reader takes as rule slugs rather than as a list of choices.
export function listsRuleSet({ values }: Pick<HelpEntry, 'values'>, slugs: ReadonlySet<string>): boolean {
  return values?.length === slugs.size && values.every(value => slugs.has(value))
}

// Appends the values an option takes to a sentence of its help text, each as
// inline code and the last joined by "or".
export function withValues(sentence: string, values?: readonly string[]): string {
  if (values === undefined) return sentence
  return `${stripSuffix(sentence, '.')}, ${VALUE_LIST.format(values.map(value => `\`${value}\``))}`
}
