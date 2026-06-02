// Derives the card-header shorthand for a lint finding from the data the
// hover already carries (rule, flagged text, message, and any fix edit).
// Two shapes cover the lint surface: a `replace` before/after pair and a
// `remove`.

interface ReplaceShorthand { after : string; before : string; kind : 'replace' }
interface RemoveShorthand  { kind  : 'remove'; text : string                   }

export type Shorthand = RemoveShorthand | ReplaceShorthand

interface ShorthandInput {
  before    ?: string
  flagged    : string
  message    : string
  rule       : string
  suggested ?: string
}

function truncate(value: string, max = 48): string {
  return value.length > max ? `${value.slice(0, max - 1)}…` : value
}

function firstBacktick(message: string): string | undefined {
  return /`([^`]+)`/.exec(message)?.[1]
}

export function lintShorthand(input: ShorthandInput): Shorthand | null {
  const { before, flagged, message, rule, suggested } = input
  switch (rule) {
    case 'legacy-union-syntax':
      return before && suggested
        ? { after : truncate(suggested), before, kind : 'replace' }
        : null
    case 'single-use-variables': {
      // `flagged` spans the binding name, leaving the inlined value to come
      // from single_use_variables.rs's "Consider inlining `<value>`" message.
      const inlined = /Consider inlining `([^`]+)`/.exec(message)?.[1]
      return flagged && inlined
        ? { after : truncate(inlined), before : flagged, kind : 'replace' }
        : null
    }
    case 'bare-imports':
      // `flagged` spans the import name, so the rewrite needs no message read.
      return flagged
        ? { after : `from ${flagged} import …`, before : `import ${flagged}`, kind : 'replace' }
        : null
    case 'reassigned-constants': {
      // The diagnostic spans the whole assignment, so the name comes from
      // the first backtick of reassigned_constants.rs's message, with the
      // lowercase rename standing in for the rule's first suggestion.
      const name = firstBacktick(message)
      return name ? { after : name.toLowerCase(), before : name, kind : 'replace' } : null
    }
    case 'step-narration':
      return { kind : 'remove', text : truncate(flagged) }
    default:
      return null
  }
}
