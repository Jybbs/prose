import { Resvg }                        from '@resvg/resvg-js'
import { createElement, type JSXNode }  from 'satori/jsx'
import satori, { type Font }            from 'satori'

export const CARD_HEIGHT = 630
export const CARD_WIDTH  = 1200

export const META_LABEL = '#8b7f9e'
export const MONO_DIM   = '#a8a0c0'

const BG         = '#16151a'
const BORDER     = 'rgba(255, 255, 255, 0.10)'
const META_VALUE = '#e8dec8'
const PANEL_FILL = 'rgba(255, 255, 255, 0.04)'

export async function rasterize(node: JSXNode, fonts: Font[]): Promise<Buffer> {
  const svg = await satori(node, { fonts, height: CARD_HEIGHT, width: CARD_WIDTH })
  return new Resvg(svg).render().asPng()
}

export function cardShell(...children: JSXNode[]): JSXNode {
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
    ...children
  )
}

export function leftRail(color: string): JSXNode {
  return createElement('div', {
    style: {
      backgroundImage : `linear-gradient(to bottom, ${color}, ${color}cc)`,
      bottom          : 0,
      left            : 50,
      position        : 'absolute',
      top             : 0,
      width           : 14
    }
  })
}

export function monoLabel(color: string, size: number, track = '0.14em') {
  return {
    color         : color,
    fontFamily    : 'JetBrains Mono',
    fontSize      : size,
    fontWeight    : 500,
    letterSpacing : track
  }
}

export function panelShell(accent: string, alpha: string, ...children: JSXNode[]): JSXNode {
  return createElement('div',
    {
      style: {
        backgroundColor : PANEL_FILL,
        border          : `1px solid ${accent}${alpha}`,
        borderRadius    : 8,
        display         : 'flex',
        flexDirection   : 'column',
        minWidth        : 360,
        padding         : '24px 28px',
        position        : 'absolute',
        right           : 80,
        top             : 80
      }
    },
    ...children
  )
}

export function panelDivider(): JSXNode {
  return createElement('div', {
    style: {
      borderTop    : `1px solid ${BORDER}`,
      height       : 1,
      marginBottom : 18,
      marginTop    : 14
    }
  })
}

function metaRow(
  label        : string,
  value        : string,
  valueStyle   : Record<string, unknown>,
  gap          : number,
  marginBottom : number = 0
): JSXNode {
  return createElement('div',
    {
      style: { alignItems: 'baseline', display: 'flex', gap, justifyContent: 'space-between', marginBottom }
    },
    createElement('div', { children: label, style: monoLabel(META_LABEL, 16) }),
    createElement('div', {
      children : value,
      style    : { color: META_VALUE, fontVariantNumeric: 'tabular-nums', ...valueStyle }
    })
  )
}

export function panelRow(label: string, value: string): JSXNode {
  return metaRow(label.toUpperCase(), value,
    { fontFamily: 'JetBrains Mono', fontSize: 19, fontWeight: 500 }, 24, 8)
}

export function versionCallout(version: string): JSXNode {
  return metaRow('VERSION', version,
    { fontFamily: 'Fraunces', fontSize: 72, fontWeight: 600, letterSpacing: '-0.01em', lineHeight: 1 }, 18)
}
