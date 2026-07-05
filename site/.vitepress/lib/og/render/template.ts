import type { JSXNode } from 'satori/jsx'

import { formatFolio }                from '../../shared/numerals'
import { CATEGORY_META, FAMILY_META } from '../../shared/registries'

import { type BrandAssets, BRAND_TITLE_ASPECT } from './assets'
import { PALETTE }                              from '../../shared/palette'
import type { OgPage }                          from '../pages'
import * as parts                               from './parts'

const DOCS_TRACK = '0.14em'

const CODE_CHIP = {
  backgroundColor : 'rgba(255, 255, 255, 0.08)',
  borderRadius    : 4,
  color           : PALETTE['ube-pale'],
  fontFamily      : 'JetBrains Mono',
  fontSize        : 19,
  padding         : '2px 8px',
  transform       : 'translateY(-2px)'
}

const TITLE_SIZES = {
  bare : [[4, 144], [8, 132], [14, 120], [Infinity, 100]],
  cap  : [[12, 108], [17, 100], [22, 84], [Infinity, 76]]
} as const

export function pageSvg(
  page    : OgPage,
  brand   : BrandAssets,
  version : string
): Promise<string> {
  return parts.toSvg(buildCard(page, version, brand.wordmark, brand.glyph), brand.fonts)
}

function buildCard(page: OgPage, version: string, wordmark: string, glyph: string): JSXNode {
  const accent = page.accent ?? PALETTE.ube
  return parts.cardShell(
    watermarkLayer(glyph),
    parts.leftRail(accent),
    wordmarkBlock(wordmark),
    dataPanel(page, version, accent),
    titleBlock(page, accent)
  )
}

function buildKicker(page: OgPage): string {
  const segments = page.breadcrumb.map(s => s.toUpperCase())
  if (page.category) {
    const tail = CATEGORY_META[page.category].label.toUpperCase()
    if (segments.at(-1) !== tail) segments.push(tail)
  }
  return `— ${segments.join(' · ')} —`
}

function dataPanel(page: OgPage, version: string, accent: string): JSXNode {
  const rows = panelRows(page)
  const warm = page.family !== undefined && FAMILY_META[page.family].warmth === 'warm'
  return parts.panelShell(accent, warm ? '99' : '66',
    ...rows.map(row => parts.panelRow(...row)),
    ...(rows.length > 0 ? [parts.panelDivider()] : []),
    parts.versionCallout(version)
  )
}

function fitTitleSize(text: string, hasCaption: boolean): number {
  return TITLE_SIZES[hasCaption ? 'cap' : 'bare'].find(([max]) => text.length <= max)![1]
}

function captionSegments(raw: string): ReadonlyArray<{ code: boolean; text: string }> {
  const segs  = [] as Array<{ code: boolean; text: string }>
  const strip = (s: string) => s.replace(/(\*\*?|_)(.+?)\1/g, '$2')
  const words = (s: string) => strip(s).split(/\s+/).filter(Boolean)
  const re    = /`([^`]+)`/g
  let last = 0
  let m: RegExpExecArray | null
  while ((m = re.exec(raw)) !== null) {
    if (m.index > last) words(raw.slice(last, m.index)).forEach(w => segs.push({ code: false, text: w }))
    segs.push({ code: true, text: m[1] })
    last = re.lastIndex
  }
  if (last < raw.length) words(raw.slice(last)).forEach(w => segs.push({ code: false, text: w }))
  return segs
}

function panelRows(page: OgPage): ReadonlyArray<readonly [string, string]> {
  if (page.kind === 'rules' && page.family !== undefined) {
    const rows: Array<[string, string]> = [['Family', page.family]]
    if (page.category && page.category !== page.family) rows.push(['Category', page.category])
    if (page.pipeline) {
      const { position, total } = page.pipeline
      rows.push(['Pipeline', `${formatFolio(position)} / ${formatFolio(total)}`])
    }
    return rows
  }
  if (page.kind === 'primitives' && page.primitive) {
    return [['Section', 'primitives'], ['Surface', page.primitive.stability]]
  }
  return []
}

function titleBlock(page: OgPage, accent: string): JSXNode {
  const caption = page.caption
  return parts.el('div',
    {
      style: {
        display       : 'flex',
        flexDirection : 'column',
        left          : 80,
        position      : 'absolute',
        right         : 80,
        top           : 360
      }
    },
    parts.el('div', {
      children : buildKicker(page),
      style    : { ...parts.monoLabel(PALETTE['ube-pale'], 22), marginBottom: 12 }
    }),
    parts.el('div', {
      children : page.title,
      style: {
        color         : accent,
        display       : 'flex',
        fontFamily    : 'Fraunces',
        fontSize      : fitTitleSize(page.title, caption !== undefined),
        fontStyle     : 'normal',
        fontWeight    : 600,
        letterSpacing : '-0.015em',
        lineHeight    : 1.02,
        marginBottom  : 14,
        maxWidth      : 1040
      }
    }),
    ...(caption !== undefined ? [parts.el('div', {
      children : captionSegments(caption).map(seg => parts.el('span', {
        children : seg.text,
        style    : seg.code ? CODE_CHIP : {}
      })),
      style : {
        alignItems : 'baseline',
        color      : PALETTE.champagne,
        columnGap  : 7,
        display    : 'flex',
        flexWrap   : 'wrap',
        fontFamily : 'Lora',
        fontSize   : 24,
        fontWeight : 400,
        maxWidth   : 1040,
        rowGap     : 10
      }
    })] : [])
  )
}

function watermarkLayer(glyph: string): JSXNode {
  const size = 720
  return parts.el('div',
    {
      style: {
        display  : 'flex',
        left     : (parts.CARD_WIDTH - size) / 2,
        opacity  : 0.012,
        position : 'absolute',
        top      : (parts.CARD_HEIGHT - size) / 2
      }
    },
    parts.el('img', { height: size, src: glyph, width: size })
  )
}

function wordmarkBlock(wordmark: string): JSXNode {
  const height = 76
  return parts.el('div',
    {
      style: {
        alignItems : 'flex-end',
        display    : 'flex',
        gap        : 10,
        left       : 80,
        position   : 'absolute',
        top        : 80
      }
    },
    parts.el('img', {
      height : height,
      src    : wordmark,
      style  : { display: 'flex' },
      width  : Math.round(height * BRAND_TITLE_ASPECT)
    }),
    parts.el('div', {
      children : 'DOCS',
      style: {
        backgroundColor : `${PALETTE.ube}2e`,
        border          : `1px solid ${PALETTE.champagne}52`,
        borderRadius    : 6,
        color           : PALETTE.champagne,
        display         : 'flex',
        fontFamily      : 'JetBrains Mono',
        fontSize        : 15,
        fontWeight      : 600,
        letterSpacing   : DOCS_TRACK,
        marginBottom    : 22,
        padding         : '6px 12px'
      }
    })
  )
}
