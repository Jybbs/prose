import fs   from 'node:fs'
import path from 'node:path'

const TOKENS_CSS = path.join(import.meta.dirname, '..', '..', 'theme', 'styles', 'tokens.css')

export function cssColor(): (token: string) => string {
  const css  = fs.readFileSync(TOKENS_CSS, 'utf8')
  const decl = (name: string) => css.match(new RegExp(`--${name}\\s*:\\s*([^;]+);`))?.[1].trim() ?? ''
  return token => {
    const alias = decl(token).match(/var\(--(.+?)\)/)?.[1]
    return alias ? decl(alias) : decl(token)
  }
}

const color = cssColor()

export const BG         = color('prose-c-woodsmoke')
export const BODY       = color('prose-c-sisal')
export const BRANDY     = color('prose-c-brandy')
export const META_LABEL = color('prose-c-waterloo')
export const META_VALUE = color('prose-c-sisal')
export const MONO_DIM   = color('prose-c-abigail')
export const UBE        = color('prose-c-ube')
