import fs                from 'node:fs'
import { createRequire } from 'node:module'
import path              from 'node:path'

import type { Font } from 'satori'

import { svgViewBox } from '../../shared/svg-view-box'
import { FONT }       from './parts'

const require = createRequire(import.meta.url)

export interface BrandImage {
  aspect : number
  src    : string
}

export interface BrandAssets {
  fonts            : Font[]
  glyph            : string
  titleWithTagline : BrandImage
  wordmark         : BrandImage
}

const FONT_FACES: readonly Omit<Font, 'data'>[] = [
  { name: FONT.display, style: 'italic', weight: 400 },
  { name: FONT.display, style: 'normal', weight: 400 },
  { name: FONT.display, style: 'italic', weight: 500 },
  { name: FONT.display, style: 'normal', weight: 600 },
  { name: FONT.display, style: 'normal', weight: 700 },
  { name: FONT.mono,    style: 'normal', weight: 400 },
  { name: FONT.mono,    style: 'normal', weight: 500 },
  { name: FONT.mono,    style: 'normal', weight: 700 },
  { name: FONT.body,    style: 'italic', weight: 400 },
  { name: FONT.body,    style: 'normal', weight: 400 }
]

export function loadBrandAssets(srcDir: string): BrandAssets {
  const fonts = FONT_FACES.map(face => ({ ...face, data: fs.readFileSync(fontFile(face)) }))
  const read  = (file: string): Buffer => fs.readFileSync(path.join(srcDir, 'public', file))
  return {
    fonts            : fonts,
    glyph            : dataUri(read('logo.svg')),
    titleWithTagline : brandImage('title-with-tagline.svg', read),
    wordmark         : brandImage('title.svg', read)
  }
}

function brandImage(file: string, read: (file: string) => Buffer): BrandImage {
  const svg        = read(file)
  const [, , w, h] = svgViewBox(svg.toString(), file).split(/\s+/).map(Number)
  return { aspect: w / h, src: dataUri(svg) }
}

function fontFile(face: Omit<Font, 'data'>): string {
  const id = face.name.toLowerCase().replaceAll(' ', '-')
  return require.resolve(`@fontsource/${id}/files/${id}-latin-${face.weight}-${face.style}.woff`)
}

function dataUri(svg: Buffer): string {
  return `data:image/svg+xml;base64,${svg.toString('base64')}`
}
