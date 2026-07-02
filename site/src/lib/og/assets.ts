import fs                from 'node:fs'
import { createRequire } from 'node:module'

import { root }      from 'astro:config/server'
import type { Font } from 'satori'

const require = createRequire(import.meta.url)

export interface BrandAssets {
  fonts            : Font[]
  glyph            : string
  titleAspect      : number
  titleWithTagline : string
  wordmark         : string
}

// The faces the card templates name. satori embeds each glyph as a vector
// path, so only these weights reach the renderer.
const FONT_FACES: readonly Omit<Font, 'data'>[] = [
  { name: 'Fraunces',       style: 'normal', weight: 600 },
  { name: 'JetBrains Mono', style: 'normal', weight: 500 },
  { name: 'JetBrains Mono', style: 'normal', weight: 700 },
  { name: 'Lora',           style: 'normal', weight: 400 }
]

export function loadBrandAssets(): BrandAssets {
  const publicDir = new URL('public/', root)
  const read      = (file: string): Buffer => fs.readFileSync(new URL(file, publicDir))
  const title     = read('title.svg')
  return {
    fonts            : FONT_FACES.map(face => ({ ...face, data: fs.readFileSync(fontFile(face)) })),
    glyph            : dataUri(read('logo.svg')),
    titleAspect      : viewBoxAspect(title),
    titleWithTagline : dataUri(read('title-with-tagline.svg')),
    wordmark         : dataUri(title)
  }
}

function dataUri(svg: Buffer): string {
  return `data:image/svg+xml;base64,${svg.toString('base64')}`
}

function fontFile(face: Omit<Font, 'data'>): string {
  const id = face.name.toLowerCase().replaceAll(' ', '-')
  return require.resolve(`@fontsource/${id}/files/${id}-latin-${face.weight}-${face.style}.woff`)
}

function viewBoxAspect(svg: Buffer): number {
  const box = svg.toString().match(/viewBox="0 0 ([\d.]+) ([\d.]+)"/)
  if (box === null) throw new Error('no viewBox on title.svg')
  return Number(box[1]) / Number(box[2])
}
