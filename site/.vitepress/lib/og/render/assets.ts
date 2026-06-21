import fs                from 'node:fs'
import { createRequire } from 'node:module'
import path              from 'node:path'

import type { Font } from 'satori'

const require = createRequire(import.meta.url)

export const BRAND_TITLE_ASPECT = 1031 / 380

export interface BrandAssets {
  fonts            : Font[]
  glyph            : string
  titleWithTagline : string
  wordmark         : string
}

const FONT_FACES: readonly Omit<Font, 'data'>[] = [
  { name: 'Fraunces',       style: 'italic', weight: 400 },
  { name: 'Fraunces',       style: 'normal', weight: 400 },
  { name: 'Fraunces',       style: 'italic', weight: 500 },
  { name: 'Fraunces',       style: 'normal', weight: 600 },
  { name: 'Fraunces',       style: 'normal', weight: 700 },
  { name: 'JetBrains Mono', style: 'normal', weight: 400 },
  { name: 'JetBrains Mono', style: 'normal', weight: 500 },
  { name: 'JetBrains Mono', style: 'normal', weight: 700 },
  { name: 'Lora',           style: 'italic', weight: 400 },
  { name: 'Lora',           style: 'normal', weight: 400 }
]

export function loadBrandAssets(srcDir: string): BrandAssets {
  const fonts = FONT_FACES.map(face => ({ ...face, data: fs.readFileSync(fontFile(face)) }))
  const glyphSvg            = fs.readFileSync(path.join(srcDir, 'public', 'logo.svg'))
  const titleWithTaglineSvg = fs.readFileSync(path.join(srcDir, 'public', 'title-with-tagline.svg'))
  const wordmarkSvg         = fs.readFileSync(path.join(srcDir, 'public', 'title.svg'))
  return {
    fonts            : fonts,
    glyph            : dataUri(glyphSvg),
    titleWithTagline : dataUri(titleWithTaglineSvg),
    wordmark         : dataUri(wordmarkSvg)
  }
}

function fontFile(face: Omit<Font, 'data'>): string {
  const id = face.name.toLowerCase().replaceAll(' ', '-')
  return require.resolve(`@fontsource/${id}/files/${id}-latin-${face.weight}-${face.style}.woff`)
}

function dataUri(svg: Buffer): string {
  return `data:image/svg+xml;base64,${svg.toString('base64')}`
}
