import type { JSXNode } from 'satori/jsx'

import { type BrandAssets, BRAND_TITLE_ASPECT } from './assets'
import { PALETTE }                              from '../../shared/palette'
import * as parts                               from './parts'

const ARTIFACT_LEFT = 120
const TITLE_TOP     = 246
const TITLE_WIDTH   = 889
const TRACK         = '0.28em'

export function landingSvg(brand: BrandAssets, version: string): Promise<string> {
  return parts.toSvg(buildLandingCard(version, brand.titleWithTagline), brand.fonts)
}

function buildLandingCard(version: string, titleWithTagline: string): JSXNode {
  return parts.cardShell(
    parts.leftRail(PALETTE.ube),
    glyphBlock(),
    dataPanel(version),
    titleArtwork(titleWithTagline)
  )
}

function dataPanel(version: string): JSXNode {
  return parts.panelShell(PALETTE.ube, '66',
    parts.panelRow('URL', 'prose.fyi'),
    parts.panelDivider(),
    parts.versionCallout(version)
  )
}

function glyphBlock(): JSXNode {
  return parts.el('div',
    {
      style: {
        alignItems : 'center',
        display    : 'flex',
        gap        : 18,
        left       : ARTIFACT_LEFT,
        position   : 'absolute',
        top        : 88
      }
    },
    pilcrowMark(),
    parts.el('div',
      {
        style: {
          display       : 'flex',
          flexDirection : 'column',
          gap           : 6
        }
      },
      parts.el('div', { children: 'WRITTEN IN RUST',   style: parts.monoLabel(PALETTE.champagne,       15, TRACK) }),
      parts.el('div', { children: 'EST. 2025',         style: parts.monoLabel(PALETTE['ube-mid'], 13, TRACK) }),
      parts.el('div', { children: 'OPEN SOURCE · MIT', style: parts.monoLabel(PALETTE['ube-mid'], 13, TRACK) })
    )
  )
}

function pilcrowMark(): JSXNode {
  return parts.el('div', {
    children: '¶',
    style: {
      alignItems     : 'center',
      color          : PALETTE.ube,
      display        : 'flex',
      fontFamily     : parts.FONT.display,
      fontSize       : 80,
      fontWeight     : 600,
      height         : 72,
      justifyContent : 'center',
      lineHeight     : 1,
      width          : 72
    }
  })
}

function titleArtwork(src: string): JSXNode {
  const height = Math.round(TITLE_WIDTH / BRAND_TITLE_ASPECT)
  return parts.el('div', {
    style: {
      display  : 'flex',
      left     : Math.round((parts.CARD_WIDTH - TITLE_WIDTH) / 2),
      position : 'absolute',
      top      : TITLE_TOP
    },
    children: parts.el('img', { height, src, width: TITLE_WIDTH })
  })
}
