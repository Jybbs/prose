import { createElement, type JSXNode } from 'satori/jsx'

import { CATEGORY_META, FAMILY_META } from '../shared/registries'

import type { OgKind, OgPage } from './pages'
import {
  BG, CARD_HEIGHT, CARD_WIDTH, META_LABEL, MONO_DIM,
  leftRail, monoLabel, panelDivider, panelRow, panelShell, versionCallout
} from './parts'

const BODY            = '#d4c8b5'
const DOCS_TRACK      = '0.14em'
const WORDMARK_ASPECT = 1031 / 380

const SECTION_ACCENTS: Partial<Record<OgKind, string>> = {
  integrations : '#b8c8a8',
  primitives   : '#a89cd8',
  reference    : '#a8b8c8',
  usage        : '#c8b8a0'
}

const TITLE_SIZES = {
  bare : [[4, 144], [8, 132], [14, 120], [Infinity, 100]],
  cap  : [[12, 108], [17, 100], [22, 84], [Infinity, 76]]
} as const

export function buildCard(page: OgPage, version: string, wordmark: string, glyph: string): JSXNode {
  const accent = pageAccent(page)
  return createElement('div',
    {
      style: {
        backgroundColor : BG,
        display         : 'flex',
        flexDirection   : 'column',
        height          : '100%',
        position        : 'relative',
        width           : '100%'
      }
    },
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

function formatCaption(raw: string): string {
  return raw.replace(/(`|\*\*?|_)(.+?)\1/g, '$2').replace(/^./, c => c.toUpperCase())
}

function pageAccent(page: OgPage): string {
  return page.family !== undefined
    ? FAMILY_META[page.family].color
    : SECTION_ACCENTS[page.kind] ?? MONO_DIM
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
  const children: JSXNode[] = [
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
    })
  ]
  if (hasCaption) {
    children.push(createElement('div', {
      children : formatCaption(page.caption!),
      style    : {
        color             : BODY,
        display           : '-webkit-box',
        fontFamily        : 'Lora',
        fontSize          : 24,
        fontWeight        : 400,
        lineHeight        : 1.3,
        marginRight       : 200,
        maxWidth          : 820,
        overflow          : 'hidden',
        WebkitBoxOrient   : 'vertical',
        WebkitLineClamp   : 2
      }
    }))
  }
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
    ...children
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
        backgroundColor : 'rgba(138, 128, 203, 0.18)',
        border          : '1px solid rgba(188, 178, 218, 0.32)',
        borderRadius    : 6,
        color           : '#bcb2da',
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
