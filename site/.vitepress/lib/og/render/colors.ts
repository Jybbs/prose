import { formatHex, interpolate } from 'culori'

import { resolveToken } from '../../shared/css-token'

// oklch blend, the same operation CSS performs for `color-mix(in oklch, a, b pct%)`
const oklchMix = (a: string, b: string, pct: number): string =>
  formatHex(interpolate([a, b], 'oklch')(pct / 100))

const ube = resolveToken('prose-c-ube')

export const BG         = resolveToken('prose-c-woodsmoke')
export const BODY       = resolveToken('prose-c-champagne')
export const KICKER     = oklchMix(ube, 'white', 36)  // --prose-c-ube-pale
export const META_LABEL = oklchMix(ube, 'white', 18)  // --prose-c-ube-mid
export const UBE        = ube
