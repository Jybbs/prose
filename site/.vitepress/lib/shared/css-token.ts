import fs   from 'node:fs'
import path from 'node:path'

const TOKENS_CSS = path.join(import.meta.dirname, '..', '..', 'theme', 'styles', 'tokens.css')

function cssColor(): (token: string) => string {
  const css  = fs.readFileSync(TOKENS_CSS, 'utf8')
  const decl = (name: string) => css.match(new RegExp(`--${name}\\s*:\\s*([^;]+);`))?.[1].trim() ?? ''
  return token => {
    const alias = decl(token).match(/var\(--(.+?)\)/)?.[1]
    return alias ? decl(alias) : decl(token)
  }
}

export const resolveToken = cssColor()
