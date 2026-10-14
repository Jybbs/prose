import type { Emblem, Fit, Motif } from './emblems'

// Draws every motif with a 0.62-unit stroke repeated along the 45-degree
// diagonal at the offsets of its size, and displaces the large copies with
// turbulence, so a stroke crossing the diagonal widens like a broad nib's. A
// small drawing takes one 1.8-unit stroke, its heavy strokes 2.5 units.
export const NIB = {
  jitter : { baseFrequency: 0.7, numOctaves: 2, scale: 0.35, seed: 4, seedStep: 17 },
  large  : [-0.55, -0.28, 0, 0.28, 0.55],
  sizes  : { large: 22 },
  small  : [-0.6, -0.3, 0, 0.3, 0.6],
  stroke : { heavy: 2.5, nib: 0.62, small: 1.8 }
} as const

// Sets the round, unfilled stroke every drawing takes in the ink color.
export const STROKE = {
  'fill'            : 'none',
  'stroke'          : 'currentColor',
  'stroke-linecap'  : 'round',
  'stroke-linejoin' : 'round'
} as const

type NibSize = 'large' | 'small'

// Returns the move of one nib copy along the diagonal, inside a drawing scaled
// by `fit` so the offset stays in grid units.
export function diagonal(fit: Fit, offset: number): string {
  return `translate(${perScale(fit, offset)} ${perScale(fit, -offset)})`
}

export function fitTransform(fit: Fit): string {
  return `translate(${fit.x} ${fit.y}) scale(${fit.scale})`
}

// Writes the markup `NibBadge.vue` renders, for the inert HTML a loader or a
// markdown rule emits where no component mounts.
export function nibBadgeHtml(emblem: Emblem, px: number, label?: string): string {
  const a11y = label ? `role="img" aria-label="${label}"` : 'aria-hidden="true"'
  return `<svg class="nib-badge" width="${px}" height="${px}" viewBox="0 0 24 24" ${a11y}>`
    + `<use href="#${nibId(emblem.motif, nibSize(px))}"></use></svg>`
}

// Names each element the sprite declares for a motif, the two symbols a badge
// uses, the paths the nib copies repeat, and the filter that roughens them.
export function nibId(motif: Motif, part: 'jitter' | 'paths' | NibSize): string {
  return `nib-${motif.label.toLowerCase()}-${part}`
}

export function nibSize(px: number): NibSize {
  return px < NIB.sizes.large ? 'small' : 'large'
}

// Divides a length in grid units by the drawing's scale, so a stroke or an
// offset keeps its weight on the grid however far `fit` scales the drawing.
export function perScale(fit: Fit, units: number): number {
  return Number((units / fit.scale).toFixed(4))
}
