type Entry = readonly [key: string | undefined, value: unknown]

const scalar = (value: unknown): boolean => value === null || typeof value !== 'object'

// Renders a JSON value with one entry per row and each object's colons in one
// column, setting a nested object or array of scalars on a single row.
export function alignedJson(value: unknown, indent = ''): string {
  if (scalar(value)) return JSON.stringify(value)
  const inner   = `${indent}  `
  const entries = Array.isArray(value)
    ? value.map((item): Entry => [undefined, item])
    : Object.entries(value as object) as Entry[]
  const [open, close] = Array.isArray(value) ? ['[', ']'] : ['{', '}']
  const entry = ([key, item]: Entry, label: (key: string) => string): string =>
    `${key === undefined ? '' : label(JSON.stringify(key))}${alignedJson(item, inner)}`

  if (entries.length === 0) return `${open}${close}`
  if (indent !== '' && entries.every(([, item]) => scalar(item))) {
    return `${open} ${entries.map(pair => entry(pair, key => `${key}: `)).join(', ')} ${close}`
  }
  const width = Math.max(...entries.map(([key]) => key === undefined ? 0 : JSON.stringify(key).length))
  const rows  = entries.map(pair => `${inner}${entry(pair, key => `${key.padEnd(width)} : `)}`)
  return `${open}\n${rows.join(',\n')}\n${indent}${close}`
}
