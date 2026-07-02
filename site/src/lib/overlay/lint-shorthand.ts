// Derives the card-header shorthand for a lint finding from the data the
// hover already carries (rule, flagged text, message, and any fix edit).
// Two shapes cover the lint surface: a `replace` before/after pair and a
// `remove`.

interface RemoveShorthand  { kind: 'remove', text: string }
interface ReplaceShorthand { after: string, before: string, kind: 'replace' }

export type Shorthand = RemoveShorthand | ReplaceShorthand

interface ShorthandInput {
  before    ?: string
  flagged    : string
  message    : string
  rule       : string
  suggested ?: string
}

export function lintShorthand(input: ShorthandInput): Shorthand | null {
  const { before, flagged, message, rule, suggested } = input
  switch (rule) {
    case 'bare-imports':
      // `flagged` spans the import name, so the rewrite needs no message read.
      return flagged
        ? { after: `from ${flagged} import …`, before: `import ${flagged}`, kind: 'replace' }
        : null
    case 'legacy-union-syntax':
      return before !== undefined && suggested !== undefined
        ? { after: truncate(suggested), before, kind: 'replace' }
        : null
    case 'reassigned-constants': {
      // The diagnostic spans the whole assignment, so the name comes from
      // the first backtick of the message, with the lowercase rename standing
      // in for the rule's first suggestion.
      const name = firstBacktick(message)
      return name === undefined ? null : { after: name.toLowerCase(), before: name, kind: 'replace' }
    }
    case 'single-use-variables': {
      // `flagged` spans the binding name, leaving the inlined value to come
      // from the rule's "Consider inlining `<value>`" message.
      const inlined = /Consider inlining `([^`]+)`/.exec(message)?.[1]
      return flagged && inlined !== undefined
        ? { after: truncate(inlined), before: flagged, kind: 'replace' }
        : null
    }
    case 'step-narration':
      return { kind: 'remove', text: truncate(flagged) }
    default:
      return null
  }
}

function firstBacktick(message: string): string | undefined {
  return /`([^`]+)`/.exec(message)?.[1]
}

function truncate(value: string, max = 48): string {
  return value.length > max ? `${value.slice(0, max - 1)}…` : value
}
