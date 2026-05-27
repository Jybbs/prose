import { CATEGORY_META, type RuleFamily } from '../shared/registries'

import type { OgKind, OgPage } from './pages'

export const CARD_HEIGHT = 630
export const CARD_WIDTH  = 1200

export interface JsxNode {
  props : Record<string, unknown>
  type  : string
}

const BG              = '#16151a'
const BODY            = '#d4c8b5'
const BORDER          = 'rgba(255, 255, 255, 0.10)'
const KICKER          = '#a8a0c0'
const META_LABEL      = '#8b7f9e'
const META_VALUE      = '#e8dec8'
const NEUTRAL         = '#a8a0c0'
const PANEL_FILL      = 'rgba(255, 255, 255, 0.04)'
const TRACK           = '0.14em'
const VERSION_C       = '#e8dec8'
const WORDMARK_ASPECT = 1031 / 380

const FAMILY_COLORS: Record<RuleFamily, string> = {
  alignment  : '#d8bc40',
  docs       : '#8cc5a3',
  formatting : '#c08597',
  lint       : '#e8876f',
  ordering   : '#7db3e0'
}

const WARM_FAMILIES: ReadonlySet<RuleFamily> = new Set(['alignment', 'formatting', 'lint'])

const SECTION_ACCENTS: Partial<Record<OgKind, string>> = {
  integrations : '#b8c8a8',
  primitives   : '#a89cd8',
  reference    : '#a8b8c8',
  usage        : '#c8b8a0'
}

export function buildCard(page: OgPage, version: string, wordmark: string, glyph: string): JsxNode {
  const accent = pageAccent(page)
  return {
    type  : 'div',
    props : {
      style    : { backgroundColor: BG, display: 'flex', flexDirection: 'column', height: '100%', position: 'relative', width: '100%' },
      children : [
        watermarkLayer(glyph),
        leftRail(accent),
        wordmarkBlock(wordmark),
        dataPanel(page, version, accent),
        titleBlock(page, accent)
      ]
    }
  }
}

function watermarkLayer(glyph: string): JsxNode {
  const size = 720
  return {
    type  : 'div',
    props : {
      style    : { display: 'flex', left: (CARD_WIDTH - size) / 2, opacity: 0.012, position: 'absolute', top: (CARD_HEIGHT - size) / 2 },
      children : { type: 'img', props: { height: size, src: glyph, width: size } }
    }
  }
}

function buildKicker(page: OgPage): string {
  const parts = page.breadcrumb.map(s => s.toUpperCase())
  if (page.category) {
    const tail = CATEGORY_META[page.category].label.toUpperCase()
    if (parts.at(-1) !== tail) parts.push(tail)
  }
  return `— ${parts.join(' · ')} —`
}

function dataPanel(page: OgPage, version: string, accent: string): JsxNode {
  const rows     = panelRows(page)
  const alpha    = page.family !== undefined && WARM_FAMILIES.has(page.family) ? '99' : '66'
  const children: JsxNode[] = rows.map(([label, value]) => panelRow(label, value))
  if (rows.length > 0) children.push({ type: 'div', props: { style: { borderTop: `1px solid ${BORDER}`, height: 1, marginBottom: 18, marginTop: 14 } } })
  children.push(versionCallout(version))
  return {
    type  : 'div',
    props : {
      style : { backgroundColor: PANEL_FILL, border: `1px solid ${accent}${alpha}`, borderRadius: 8, display: 'flex', flexDirection: 'column', minWidth: 360, padding: '24px 28px', position: 'absolute', right: 80, top: 80 },
      children
    }
  }
}

function fitTitleSize(text: string, hasCaption: boolean): number {
  if (!hasCaption) {
    if (text.length <= 4)  return 144
    if (text.length <= 8)  return 132
    if (text.length <= 14) return 120
    return 100
  }
  if (text.length > 22) return 76
  if (text.length > 17) return 84
  if (text.length > 12) return 100
  return 108
}

function formatCaption(raw: string): string {
  const s = raw
    .replace(/`([^`]+)`/g,    '$1')
    .replace(/\*\*([^*]+)\*\*/g, '$1')
    .replace(/\*([^*]+)\*/g,  '$1')
    .replace(/_([^_]+)_/g,    '$1')
  return s.length === 0 ? s : s[0].toUpperCase() + s.slice(1)
}

function leftRail(color: string): JsxNode {
  return {
    type  : 'div',
    props : {
      style : { backgroundImage: `linear-gradient(to bottom, ${color}, ${color}cc)`, bottom: 0, left: 50, position: 'absolute', top: 0, width: 14 }
    }
  }
}

function monoLabel(color: string, size: number) {
  return { color, fontFamily: 'JetBrains Mono', fontSize: size, fontWeight: 500, letterSpacing: TRACK }
}

function pageAccent(page: OgPage): string {
  if (page.family !== undefined) return FAMILY_COLORS[page.family]
  return SECTION_ACCENTS[page.kind] ?? NEUTRAL
}

function panelRow(label: string, value: string): JsxNode {
  return {
    type  : 'div',
    props : {
      style    : { alignItems: 'baseline', display: 'flex', gap: 24, justifyContent: 'space-between', marginBottom: 8 },
      children : [
        { type: 'div', props: { children: label.toUpperCase(), style: monoLabel(META_LABEL, 16) } },
        { type: 'div', props: { children: value, style: { color: META_VALUE, fontFamily: 'JetBrains Mono', fontSize: 19, fontVariantNumeric: 'tabular-nums', fontWeight: 500 } } }
      ]
    }
  }
}

function panelRows(page: OgPage): ReadonlyArray<readonly [string, string]> {
  if (page.kind === 'rules' && page.family !== undefined) {
    const rows: Array<[string, string]> = [['Family', page.family]]
    if (page.category && page.category !== page.family) rows.push(['Category', page.category])
    if (page.pipeline) rows.push(['Pipeline', `${String(page.pipeline.position).padStart(2, '0')} / ${String(page.pipeline.total).padStart(2, '0')}`])
    return rows
  }
  if (page.kind === 'primitives' && page.primitive) {
    return [['Section', 'primitives'], ['Surface', page.primitive.stability]]
  }
  return []
}

function titleBlock(page: OgPage, accent: string): JsxNode {
  const hasCaption = page.caption !== undefined
  const kickerText = buildKicker(page)
  const titleSize  = fitTitleSize(page.title, hasCaption)
  const children: JsxNode[] = [
    { type: 'div', props: { children: kickerText, style: { ...monoLabel(KICKER, 22), marginBottom: 12 } } },
    { type: 'div', props: { children: page.title, style: { color: accent, display: 'flex', fontFamily: 'Fraunces', fontSize: titleSize, fontStyle: 'normal', fontWeight: 600, letterSpacing: '-0.015em', lineHeight: 1.02, marginBottom: 14, maxWidth: 1040 } } }
  ]
  if (hasCaption) children.push({ type: 'div', props: { children: formatCaption(page.caption!), style: { color: BODY, display: '-webkit-box', fontFamily: 'Lora', fontSize: 24, fontWeight: 400, lineHeight: 1.3, marginRight: 200, maxWidth: 820, overflow: 'hidden', WebkitBoxOrient: 'vertical', WebkitLineClamp: 2 } } })
  return {
    type  : 'div',
    props : {
      style : { display: 'flex', flexDirection: 'column', left: 80, position: 'absolute', right: 80, top: 360 },
      children
    }
  }
}

function versionCallout(version: string): JsxNode {
  return {
    type  : 'div',
    props : {
      style    : { alignItems: 'baseline', display: 'flex', gap: 18, justifyContent: 'space-between' },
      children : [
        { type: 'div', props: { children: 'VERSION', style: monoLabel(META_LABEL, 16) } },
        { type: 'div', props: { children: version, style: { color: VERSION_C, fontFamily: 'Fraunces', fontSize: 72, fontVariantNumeric: 'tabular-nums', fontWeight: 600, letterSpacing: '-0.01em', lineHeight: 1 } } }
      ]
    }
  }
}

function wordmarkBlock(wordmark: string): JsxNode {
  const height = 76
  return {
    type  : 'div',
    props : {
      style    : { alignItems: 'flex-end', display: 'flex', gap: 10, left: 80, position: 'absolute', top: 80 },
      children : [
        { type: 'img', props: { height, src: wordmark, style: { display: 'flex' }, width: Math.round(height * WORDMARK_ASPECT) } },
        { type: 'div', props: { children: 'DOCS', style: { backgroundColor: 'rgba(138, 128, 203, 0.18)', border: '1px solid rgba(188, 178, 218, 0.32)', borderRadius: 6, color: '#bcb2da', display: 'flex', fontFamily: 'JetBrains Mono', fontSize: 15, fontWeight: 600, letterSpacing: TRACK, marginBottom: 22, padding: '6px 12px' } } }
      ]
    }
  }
}
