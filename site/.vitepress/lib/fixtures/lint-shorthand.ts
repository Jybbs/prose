// Derives the card-header shorthand for a lint finding from the data the
// hover already carries (rule, flagged text, message, and any fix edit).
// Three shapes cover the lint surface: a `replace` before/after pair, a
// `relocate` of a constant into one of a few homes, and a `remove`.

export interface RelocateHome {
  keyword : boolean
  leaf    : string
  parent  : string
}

export const LOOSE_CONSTANT_HOMES: readonly RelocateHome[] = [
  { keyword : false, leaf : 'member', parent : 'enum'  },
  { keyword : true,  leaf : 'field',  parent : 'class' },
  { keyword : true,  leaf : 'local',  parent : 'def'   }
]

export interface ReplaceShorthand  { after : string; before : string; kind : 'replace'  }
export interface RelocateShorthand { kind  : 'relocate'; name : string                  }
export interface RemoveShorthand   { kind  : 'remove'; text : string                    }

export type Shorthand = RelocateShorthand | RemoveShorthand | ReplaceShorthand

export interface ShorthandInput {
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
      const name    = firstBacktick(message)
      const inlined = /Consider inlining `([^`]+)`/.exec(message)?.[1]
      return name && inlined
        ? { after : truncate(inlined), before : name, kind : 'replace' }
        : null
    }
    case 'bare-import-allowlist': {
      const module = firstBacktick(message)
      return module
        ? { after : `from ${module} import …`, before : `import ${module}`, kind : 'replace' }
        : null
    }
    case 'loose-constants': {
      const name = firstBacktick(message)
      return name ? { kind : 'relocate', name } : null
    }
    case 'no-step-narration':
      return { kind : 'remove', text : truncate(flagged) }
    default:
      return null
  }
}
