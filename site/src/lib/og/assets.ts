import fs                from 'node:fs'
import { createRequire } from 'node:module'

import { root }      from 'astro:config/server'
import type { Font } from 'satori'

import { FONTS } from '../tokens/fonts'

const require = createRequire(import.meta.url)

export interface BrandAssets {
  fonts            : Font[]
  glyph            : string
  titleAspect      : number
  titleWithTagline : string
  wordmark         : string
}

export function loadBrandAssets(): BrandAssets {
  const publicDir = new URL('public/', root)
  const read      = (file: string): Buffer => fs.readFileSync(new URL(file, publicDir))
  const title     = read('title.svg')
  return {
    fonts            : cardFonts(),
    glyph            : dataUri(read('logo.svg')),
    titleAspect      : viewBoxAspect(title),
    titleWithTagline : dataUri(read('title-with-tagline.svg')),
    wordmark         : dataUri(title)
  }
}

// satori embeds each glyph as a vector path, so only the static weights from
// each face's `@fontsource` package reach the renderer.
function cardFonts(): Font[] {
  return Object.values(FONTS).flatMap(face =>
    face.staticWeights.map(weight => ({
      data   : fs.readFileSync(fontFile(face.slug, weight)),
      name   : face.name,
      style  : 'normal' as const,
      weight
    }))
  )
}

function dataUri(svg: Buffer): string {
  return `data:image/svg+xml;base64,${svg.toString('base64')}`
}

function fontFile(slug: string, weight: number): string {
  return require.resolve(`@fontsource/${slug}/files/${slug}-latin-${weight}-normal.woff`)
}

function viewBoxAspect(svg: Buffer): number {
  const box = svg.toString().match(/viewBox="0 0 ([\d.]+) ([\d.]+)"/)
  if (box === null) throw new Error('no viewBox on title.svg')
  return Number(box[1]) / Number(box[2])
}
