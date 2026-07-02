import type { JSXNode } from 'satori/jsx'

import { type BrandAssets, BRAND_TITLE_ASPECT } from './assets'
import { BODY, META_LABEL, UBE }                from './colors'
import {
  CARD_WIDTH,
  cardShell, dataPanel, el, leftRail, monoLabel, toSvg
} from './parts'

const ARTIFACT_LEFT = 120
const TITLE_TOP     = 246
const TITLE_WIDTH   = 889
const TRACK         = '0.28em'

export function landingSvg(brand: BrandAssets, version: string): Promise<string> {
  return toSvg(buildLandingCard(version, brand.titleWithTagline), brand.fonts)
}

function buildLandingCard(version: string, titleWithTagline: string): JSXNode {
  return cardShell(
    leftRail(UBE),
    glyphBlock(),
    dataPanel(UBE, '66', [['URL', 'prose.fyi']], version),
    titleArtwork(titleWithTagline)
  )
}

function glyphBlock(): JSXNode {
  return el('div',
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
    el('div',
      {
        style: {
          display       : 'flex',
          flexDirection : 'column',
          gap           : 6
        }
      },
      el('div', { children: 'WRITTEN IN RUST',   style: monoLabel(BODY,       15, TRACK) }),
      el('div', { children: 'EST. 2025',         style: monoLabel(META_LABEL, 13, TRACK) }),
      el('div', { children: 'OPEN SOURCE · MIT', style: monoLabel(META_LABEL, 13, TRACK) })
    )
  )
}

function pilcrowMark(): JSXNode {
  return el('div', {
    children : '¶',
    style    : {
      alignItems     : 'center',
      color          : UBE,
      display        : 'flex',
      fontFamily     : 'Fraunces',
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
  return el('div', {
    children : el('img', { height, src, width: TITLE_WIDTH }),
    style    : {
      display  : 'flex',
      left     : Math.round((CARD_WIDTH - TITLE_WIDTH) / 2),
      position : 'absolute',
      top      : TITLE_TOP
    }
  })
}
