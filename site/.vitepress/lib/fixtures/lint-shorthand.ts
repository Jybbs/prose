// Derives the card-header shorthand for a lint finding from the data the
// hover already carries (rule, flagged text, message, and the fix edit's
// two sides). Four shapes cover the lint surface: a `replace` before/after
// pair, a `remove`, an `insert` against an empty before, and a `block`
// whose replacement spans several lines.

interface BlockShorthand   { after  : string, before   : string, kind : 'block'   }
interface InsertShorthand  { anchor : string, inserted : string, kind : 'insert'  }
interface RemoveShorthand  { kind   : 'remove', text   : string                   }
interface ReplaceShorthand { after  : string, before   : string, kind : 'replace' }

export type Shorthand = BlockShorthand | InsertShorthand | RemoveShorthand | ReplaceShorthand

interface ShorthandInput {
  flagged    : string
  message    : string
  replaced  ?: string
  rule       : string
  suggested ?: string
}

function truncate(value: string, max = 48): string {
  return value.length > max ? `${value.slice(0, max - 1)}…` : value
}

function truncateLines(value: string, max = 10): string {
  const lines = value.split('\n')
  return lines.length > max ? `${lines.slice(0, max).join('\n')}\n…` : value
}

function firstBacktick(message: string): string | undefined {
  return /`([^`]+)`/.exec(message)?.[1]
}

// Picks the rendering shape from the replacement itself, so a suggestion
// spanning several lines reads as stacked code, capped to what the panes
// show, and a single-token one keeps the inline chip pair.
function replacement(before: string, after: string): Shorthand {
  return before.includes('\n') || after.includes('\n')
    ? { after : truncateLines(after), before : truncateLines(before), kind : 'block' }
    : { after, before, kind : 'replace' }
}

export function lintShorthand(input: ShorthandInput): Shorthand | null {
  const { flagged, message, replaced, rule, suggested } = input
  switch (rule) {
    case 'bare-imports':
      // `flagged` spans the import name, so the rewrite needs no message read.
      return flagged
        ? replacement(`import ${flagged}`, `from ${flagged} import …`)
        : null
    case 'line-overflow':
      // The split rewrites one literal rather than the whole flagged line, so
      // the pair comes from the edit's own two sides.
      return replaced && suggested ? replacement(replaced, suggested) : null
    case 'miscased-constants':
      // `flagged` spans the miscased name and `suggested` carries the
      // SCREAMING_CASE rename from the display-only fix.
      return flagged && suggested
        ? replacement(flagged, truncate(suggested))
        : null
    case 'reassigned-constants': {
      // The diagnostic spans the whole assignment, so the name comes from
      // the first backtick of reassigned_constants.rs's message, with the
      // lowercase rename standing in for the rule's first suggestion.
      const name = firstBacktick(message)
      return name ? replacement(name, name.toLowerCase()) : null
    }
    case 'signature-annotations':
      // The fix inserts against an empty before, so `flagged` anchors the
      // parameter name and `suggested` carries the annotation it takes.
      return flagged && suggested
        ? { anchor : flagged, inserted : truncate(suggested), kind : 'insert' }
        : null
    case 'inlinable-bindings': {
      // `flagged` spans the binding name, leaving the inlined value to come
      // from the rule's "Consider inlining `<value>`" message.
      const inlined = /Consider inlining `([^`]+)`/.exec(message)?.[1]
      return flagged && inlined
        ? replacement(flagged, truncate(inlined))
        : null
    }
    case 'step-narration':
      return { kind : 'remove', text : truncate(flagged) }
    default:
      return null
  }
}
