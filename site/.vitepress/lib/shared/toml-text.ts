import { stringify } from 'smol-toml'

// A config table rendered as TOML text, a table carrying nothing collapsing to
// the empty string rather than to whitespace.
export function tomlText(value: Record<string, unknown>): string {
  const text = stringify(value)
  return text.trim() ? text : ''
}
