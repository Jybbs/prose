import fs   from 'node:fs'
import path from 'node:path'

import type { Font } from 'satori'

export interface BrandAssets {
  fonts    : Font[]
  glyph    : string
  wordmark : string
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

export function loadBrandAssets(srcDir: string, repo: string): BrandAssets {
  const fonts = FONT_FACES.map(face => ({
    ...face,
    data: fs.readFileSync(path.join(repo, 'node_modules', '@fontsource', fontFile(face)))
  }))
  const glyphSvg    = fs.readFileSync(path.join(srcDir, 'public', 'logo.svg'))
  const wordmarkSvg = fs.readFileSync(path.join(srcDir, 'public', 'title.svg'))
  return { fonts, glyph: dataUri(glyphSvg), wordmark: dataUri(wordmarkSvg) }
}

function fontFile(face: Omit<Font, 'data'>): string {
  const id = face.name.toLowerCase().replaceAll(' ', '-')
  return `${id}/files/${id}-latin-${face.weight}-${face.style}.woff`
}

function dataUri(svg: Buffer): string {
  return `data:image/svg+xml;base64,${svg.toString('base64')}`
}
