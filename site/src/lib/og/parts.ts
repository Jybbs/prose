import { createElement as h }       from 'satori/jsx'
import type { JSXElement, JSXNode } from 'satori/jsx'
import satori, { type Font }        from 'satori'

import { BG, BODY, META_LABEL } from './colors'

export const CARD_HEIGHT = 630
export const CARD_WIDTH  = 1200

const BORDER     = 'rgba(255, 255, 255, 0.10)'
const PANEL_FILL = 'rgba(255, 255, 255, 0.04)'

// satori's createElement<P> returns JSXElement<P>, which its own JSXNode union
// rejects for non-unknown P, so el returns satori's JSX.Element (JSXElement<any, any>).
export function el(
  type       : string,
  props      : Record<string, unknown> | null,
  ...children: JSXNode[]
): JSXElement<any, any> {
  return h(type, props, ...children)
}

export function toSvg(node: JSXNode, fonts: Font[]): Promise<string> {
  return satori(node, { fonts, height: CARD_HEIGHT, width: CARD_WIDTH })
}

export function cardShell(...children: JSXNode[]): JSXNode {
  return el('div',
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
  return el('div', {
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

// The data panel every card carries: the label-value rows, a divider when any
// row exists, and the version callout beneath.
export function dataPanel(
  accent  : string,
  alpha   : string,
  rows    : ReadonlyArray<readonly [string, string]>,
  version : string
): JSXNode {
  return panelShell(accent, alpha,
    ...rows.map(row => panelRow(...row)),
    ...(rows.length > 0 ? [panelDivider()] : []),
    versionCallout(version)
  )
}

function panelShell(accent: string, alpha: string, ...children: JSXNode[]): JSXNode {
  return el('div',
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

function panelDivider(): JSXNode {
  return el('div', {
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
  return el('div',
    {
      style: { alignItems: 'baseline', display: 'flex', gap, justifyContent: 'space-between', marginBottom }
    },
    el('div', { children: label, style: monoLabel(META_LABEL, 16) }),
    el('div', {
      children : value,
      style    : { color: BODY, fontVariantNumeric: 'tabular-nums', ...valueStyle }
    })
  )
}

function panelRow(label: string, value: string): JSXNode {
  return metaRow(label.toUpperCase(), value,
    { fontFamily: 'JetBrains Mono', fontSize: 19, fontWeight: 500 }, 24, 8)
}

function versionCallout(version: string): JSXNode {
  return metaRow('VERSION', version,
    { fontFamily: 'Fraunces', fontSize: 72, fontWeight: 600, letterSpacing: '-0.01em', lineHeight: 1 }, 18)
}
