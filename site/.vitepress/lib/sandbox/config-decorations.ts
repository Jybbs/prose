import type { DecorationItem } from '@shikijs/types'

import type { LintFinding }        from '../fixtures/lint-findings'
import { lintDecorations }         from '../markdown/lint-decorations'
import type { Facet, RuleControl } from './config-schema.data'

const NOTICE = /^warning: unknown key `(?<path>[^`]+)` in \[tool\.prose\]$/u

// A slug matching no rule, so the formatted panel's flag popper skips these
// anchors when it looks one up.
export const CONFIG_SLUG = 'config-key'

// One assignment read off a `prose.toml` line, carrying the dotted path it binds
// and the columns its key text occupies.
interface KeySpan {
  end   : number
  path  : string
  row   : number
  start : number
}

// Drops the quotes a TOML key may be written in, so a path built from the source
// matches the one the deserialization reports.
function bare(segment: string): string {
  const trimmed = segment.trim()
  return /^(?<quote>["'])(?<inner>.*)\k<quote>$/u.exec(trimmed)?.groups?.inner ?? trimmed
}

// How many single-character edits separate two spellings. The ranking below
// reads this to order a rule's facets against what the reader wrote.
function distance(from: string, to: string): number {
  let row = Array.from({ length: to.length + 1 }, (_, index) => index)
  for (let i = 0; i < from.length; i += 1) {
    const next = [i + 1]
    for (let j = 0; j < to.length; j += 1) {
      next.push(Math.min(row[j] + (from[i] === to[j] ? 0 : 1), row[j + 1] + 1, next[j] + 1))
    }
    row = next
  }
  return row[to.length]
}

// Rejoins a dotted key with every segment unquoted.
function dotted(key: string): string {
  return key.split('.').map(bare).join('.')
}

// The header a `[table]` or `[[array]]` line opens, `null` for every other line.
function header(line: string): string | null {
  const name = /^\s*\[\[?(?<name>[^\]]+)\]\]?\s*(?:#.*)?$/u.exec(line)?.groups?.name
  return name === undefined ? null : dotted(name)
}

// The dotted path and key columns of every assignment in `toml`, tracking the
// table each line sits under.
function keySpans(toml: string): KeySpan[] {
  const spans: KeySpan[] = []
  let table = ''
  toml.split('\n').forEach((line, row) => {
    const opened = header(line)
    if (opened !== null) {
      table = opened
      return
    }
    const assigned = /^(?<lead>\s*)(?<key>[^=#[\]]+?)\s*=/u.exec(line)?.groups
    if (!assigned) return
    const path = dotted(assigned.key)
    spans.push({
      end   : assigned.lead.length + assigned.key.length,
      path  : table ? `${table}.${path}` : path,
      row,
      start : assigned.lead.length
    })
  })
  return spans
}

// The shiki decorations the config panel paints its unknown keys with.
export function configDecorations(notices: readonly string[], toml: string): DecorationItem[] {
  return lintDecorations(unknownKeyFindings(notices, toml), toml)
}

// The rows carrying a key prose did not recognize. The panel marks each one in
// its margin.
export function flaggedRows(notices: readonly string[], toml: string): readonly number[] {
  return [...new Set(unknownKeyFindings(notices, toml).map(finding => finding.location.row))]
}

// The key a notice names, `null` for a notice of another shape.
export function noticeKey(notice: string): string | null {
  return NOTICE.exec(notice)?.groups?.path ?? null
}

// Ranks a rule's facets by how close each spelling sits to the key the reader
// wrote, so the likeliest correction comes first. Returns nothing where the
// path names no rule, which is the case for a root key and a misspelled rule.
export function rankedFacets(path: string, rules: readonly RuleControl[]): Facet[] {
  const [head, slug, wrote] = path.split('.')
  if (head !== 'rules' || wrote === undefined) return []
  const rule = rules.find(control => control.slug === slug)
  return rule
    ? [...rule.facets].toSorted((a, b) =>
        distance(wrote, a.key) - distance(wrote, b.key) || a.key.localeCompare(b.key))
    : []
}

// Locates each unknown-key notice in the source that raised it, reporting the
// span in the shape the lint decorations read. The config panel then paints the
// same squiggle the formatted panel paints. A notice the source cannot place
// falls back to the message list, and so does a notice of another shape.
export function unknownKeyFindings(notices: readonly string[], toml: string): LintFinding[] {
  const spans = keySpans(toml)
  return notices.flatMap(notice => {
    const path = noticeKey(notice)
    const span = path === null ? undefined : spans.find(each => each.path === path)
    return span
      ? [{
          code         : CONFIG_SLUG,
          end_location : { column: span.end + 1,   row: span.row + 1 },
          location     : { column: span.start + 1, row: span.row + 1 },
          message      : notice
        }]
      : []
  })
}
