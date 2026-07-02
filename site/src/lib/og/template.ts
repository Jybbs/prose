import type { JSXNode } from 'satori/jsx'

import { FONTS }             from '../tokens/fonts'
import type { BrandAssets }  from './assets'
import { BODY, KICKER, UBE } from './colors'
import type { OgPage }       from './pages'
import {
  CARD_HEIGHT, CARD_WIDTH,
  cardShell, dataPanel, el, leftRail, monoLabel, toSvg
} from './parts'

const CODE_CHIP = {
  ...monoLabel(KICKER, 19),
  backgroundColor : 'rgba(255, 255, 255, 0.08)',
  borderRadius    : 4,
  padding         : '2px 8px',
  transform       : 'translateY(-2px)'
}

// Each pair is a max title length and the font size used up to it, `cap` for
// titles with a caption.
const TITLE_SIZES = {
  bare : [[4, 144], [8, 132], [14, 120], [Infinity, 100]],
  cap  : [[12, 108], [17, 100], [22, 84], [Infinity, 76]]
} as const

export function pageSvg(
  page    : OgPage,
  brand   : BrandAssets,
  version : string
): Promise<string> {
  return toSvg(buildCard(page, version, brand), brand.fonts)
}

function buildCard(page: OgPage, version: string, brand: BrandAssets): JSXNode {
  const accent = page.accent ?? UBE
  return cardShell(
    watermarkLayer(brand.glyph),
    leftRail(accent),
    wordmarkBlock(brand),
    dataPanel(accent, page.warmth === 'warm' ? '99' : '66', panelRows(page), version),
    titleBlock(page, accent)
  )
}

function buildKicker(page: OgPage): string {
  return `— ${page.breadcrumb.map(part => part.toUpperCase()).join(' · ')} —`
}

function captionSegments(raw: string): ReadonlyArray<{ code: boolean, text: string }> {
  const strip = (s: string): string => s.replace(/(\*\*?|_)(.+?)\1/g, '$2')
  // `split` interleaves the backtick captures at odd indexes
  return raw.split(/`([^`]+)`/).flatMap((part, index): Array<{ code: boolean, text: string }> =>
    index % 2 === 1
      ? [{ code: true, text: part }]
      : strip(part).split(/\s+/).filter(Boolean).map(text => ({ code: false, text }))
  )
}

function fitTitleSize(text: string, hasCaption: boolean): number {
  return TITLE_SIZES[hasCaption ? 'cap' : 'bare'].find(([max]) => text.length <= max)![1]
}

function formatFolio(n: number): string {
  return String(n).padStart(2, '0')
}

function panelRows(page: OgPage): ReadonlyArray<readonly [string, string]> {
  if (page.kind === 'rules' && page.family !== undefined) {
    const rows: Array<readonly [string, string]> = [['Family', page.family]]
    if (page.pipeline !== undefined) {
      const { position, total } = page.pipeline
      rows.push(['Pipeline', `${formatFolio(position)} / ${formatFolio(total)}`])
    }
    return rows
  }
  if (page.kind === 'primitives' && page.stability !== undefined) {
    return [['Section', 'primitives'], ['Surface', page.stability]]
  }
  return []
}

function titleBlock(page: OgPage, accent: string): JSXNode {
  const caption = page.caption
  return el('div',
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
    el('div', {
      children : buildKicker(page),
      style    : { ...monoLabel(KICKER, 22), marginBottom: 12 }
    }),
    el('div', {
      children : page.title,
      style    : {
        color         : accent,
        fontFamily    : FONTS.display.name,
        fontSize      : fitTitleSize(page.title, caption !== undefined),
        fontStyle     : 'normal',
        fontWeight    : 600,
        letterSpacing : '-0.015em',
        lineHeight    : 1.02,
        marginBottom  : 14,
        maxWidth      : 1040
      }
    }),
    ...(caption !== undefined ? [el('div', {
      children: captionSegments(caption).map(seg => el('span', {
        children : seg.text,
        style    : seg.code ? CODE_CHIP : {}
      })),

      style: {
        alignItems : 'baseline',
        color      : BODY,
        columnGap  : 7,
        display    : 'flex',
        flexWrap   : 'wrap',
        fontFamily : FONTS.base.name,
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
  return el('div',
    {
      style: {
        display  : 'flex',
        left     : (CARD_WIDTH - size) / 2,
        opacity  : 0.012,
        position : 'absolute',
        top      : (CARD_HEIGHT - size) / 2
      }
    },
    el('img', { height: size, src: glyph, width: size })
  )
}

function wordmarkBlock(brand: BrandAssets): JSXNode {
  const height = 76
  return el('div',
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
    el('img', {
      height : height,
      src    : brand.wordmark,
      width  : Math.round(height * brand.titleAspect)
    }),
    el('div', {
      children : 'DOCS',
      style    : {
        ...monoLabel(BODY, 15),
        backgroundColor : `${UBE}2e`,
        border          : `1px solid ${BODY}52`,
        borderRadius    : 6,
        fontWeight      : 700,
        marginBottom    : 22,
        padding         : '6px 12px'
      }
    })
  )
}
