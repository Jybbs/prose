import fs   from 'node:fs'
import path from 'node:path'

import { formatHex, interpolate } from 'culori'

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

// oklch blend, the same operation CSS performs for `color-mix(in oklch, a, b pct%)`
const oklchMix = (a: string, b: string, pct: number): string =>
  formatHex(interpolate([a, b], 'oklch')(pct / 100))

const ube = resolveToken('prose-c-ube')

export const BG         = resolveToken('prose-c-woodsmoke')
export const BODY       = resolveToken('prose-c-champagne')
export const KICKER     = oklchMix(ube, 'white', 36)  // --prose-c-ube-pale
export const META_LABEL = oklchMix(ube, 'white', 18)  // --prose-c-ube-mid
export const UBE        = ube
