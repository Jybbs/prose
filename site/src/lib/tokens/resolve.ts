import fs   from 'node:fs'
import path from 'node:path'

const TOKENS_CSS = path.join(import.meta.dirname, '..', '..', 'styles', 'tokens.css')
const css        = fs.readFileSync(TOKENS_CSS, 'utf8')

function declared(name: string): string {
  return css.match(new RegExp(`--${name}\\s*:\\s*([^;]+);`))?.[1].trim() ?? ''
}

// Reads a `--prose-*` custom property's value from the token source at build
// time, following one `var()` alias. A `color-mix()` value resolves to its
// first referenced token, leaving the concrete blend to the color consumer.
export function resolveToken(token: string): string {
  const alias = declared(token).match(/var\(--(.+?)\)/)?.[1]
  return alias ? declared(alias) : declared(token)
}
