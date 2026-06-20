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

export const BG         = resolveToken('prose-c-woodsmoke')
export const BODY       = resolveToken('prose-c-sisal')
export const BRANDY     = resolveToken('prose-c-brandy')
export const META_LABEL = resolveToken('prose-c-waterloo')
export const META_VALUE = resolveToken('prose-c-sisal')
export const MONO_DIM   = resolveToken('prose-c-abigail')
export const UBE        = resolveToken('prose-c-ube')
