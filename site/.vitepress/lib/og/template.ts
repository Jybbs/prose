import { createElement, type JSXNode } from 'satori/jsx'

import { CATEGORY_META, FAMILY_META } from '../shared/registries'

import type { BrandAssets }            from './assets'
import { BODY, BRANDY, MONO_DIM, UBE } from './colors'
import type { OgPage }                 from './pages'
import {
  CARD_HEIGHT, CARD_WIDTH,
  cardShell, leftRail, monoLabel, panelDivider, panelRow, panelShell, toSvg, versionCallout
} from './parts'

const DOCS_TRACK      = '0.14em'
const WORDMARK_ASPECT = 1031 / 380

const CODE_CHIP = {
  backgroundColor : 'rgba(255, 255, 255, 0.08)',
  borderRadius    : 4,
  color           : UBE,
  fontFamily      : 'JetBrains Mono',
  fontSize        : 19,
  padding         : '2px 8px'
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
  return toSvg(buildCard(page, version, brand.wordmark, brand.glyph), brand.fonts)
}

function buildCard(page: OgPage, version: string, wordmark: string, glyph: string): JSXNode {
  const accent = page.accent ?? UBE
  return cardShell(
    watermarkLayer(glyph),
    leftRail(accent),
    wordmarkBlock(wordmark),
    dataPanel(page, version, accent),
    titleBlock(page, accent)
  )
}

function buildKicker(page: OgPage): string {
  const parts = page.breadcrumb.map(s => s.toUpperCase())
  if (page.category) {
    const tail = CATEGORY_META[page.category].label.toUpperCase()
    if (parts.at(-1) !== tail) parts.push(tail)
  }
  return `— ${parts.join(' · ')} —`
}

function dataPanel(page: OgPage, version: string, accent: string): JSXNode {
  const rows = panelRows(page)
  const warm = page.family !== undefined && FAMILY_META[page.family].warmth === 'warm'
  return panelShell(accent, warm ? '99' : '66',
    ...rows.map(row => panelRow(...row)),
    ...(rows.length > 0 ? [panelDivider()] : []),
    versionCallout(version)
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
      const pad = (n: number) => String(n).padStart(2, '0')
      rows.push(['Pipeline', `${pad(page.pipeline.position)} / ${pad(page.pipeline.total)}`])
    }
    return rows
  }
  if (page.kind === 'primitives' && page.primitive) {
    return [['Section', 'primitives'], ['Surface', page.primitive.stability]]
  }
  return []
}

function titleBlock(page: OgPage, accent: string): JSXNode {
  const hasCaption = page.caption !== undefined
  return createElement('div',
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
    createElement('div', {
      children : buildKicker(page),
      style    : { ...monoLabel(MONO_DIM, 22), marginBottom: 12 }
    }),
    createElement('div', {
      children : page.title,
      style    : {
        color         : accent,
        display       : 'flex',
        fontFamily    : 'Fraunces',
        fontSize      : fitTitleSize(page.title, hasCaption),
        fontStyle     : 'normal',
        fontWeight    : 600,
        letterSpacing : '-0.015em',
        lineHeight    : 1.02,
        marginBottom  : 14,
        maxWidth      : 1040
      }
    }),
    ...(hasCaption ? [createElement('div', {
      children : captionSegments(page.caption!).map(seg => createElement('span', {
        children : seg.text,
        style    : seg.code ? CODE_CHIP : {}
      })),
      style : {
        alignItems : 'center',
        color      : BODY,
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
  return createElement('div',
    {
      style: {
        display  : 'flex',
        left     : (CARD_WIDTH - size) / 2,
        opacity  : 0.012,
        position : 'absolute',
        top      : (CARD_HEIGHT - size) / 2
      }
    },
    createElement('img', { height: size, src: glyph, width: size })
  )
}

function wordmarkBlock(wordmark: string): JSXNode {
  const height = 76
  return createElement('div',
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
    createElement('img', {
      height : height,
      src    : wordmark,
      style  : { display: 'flex' },
      width  : Math.round(height * WORDMARK_ASPECT)
    }),
    createElement('div', {
      children : 'DOCS',
      style    : {
        backgroundColor : `${UBE}2e`,
        border          : `1px solid ${BRANDY}52`,
        borderRadius    : 6,
        color           : BRANDY,
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
