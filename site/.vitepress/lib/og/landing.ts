import { createElement, type JSXNode }  from 'satori/jsx'

import { type BrandAssets, BRAND_TITLE_ASPECT } from './assets'
import { META_LABEL, MONO_DIM, UBE }            from './colors'
import {
  CARD_WIDTH,
  cardShell, leftRail, monoLabel, panelDivider, panelRow, panelShell, toSvg, versionCallout
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
    dataPanel(version),
    titleArtwork(titleWithTagline)
  )
}

function dataPanel(version: string): JSXNode {
  return panelShell(UBE, '66',
    panelRow('URL', 'prose.fyi'),
    panelDivider(),
    versionCallout(version)
  )
}

function glyphBlock(): JSXNode {
  return createElement('div',
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
    createElement('div',
      {
        style: {
          display       : 'flex',
          flexDirection : 'column',
          gap           : 6
        }
      },
      createElement('div', { children: 'WRITTEN IN RUST',   style: monoLabel(MONO_DIM,   15, TRACK) }),
      createElement('div', { children: 'EST. 2025',         style: monoLabel(META_LABEL, 13, TRACK) }),
      createElement('div', { children: 'OPEN SOURCE · MIT', style: monoLabel(META_LABEL, 13, TRACK) })
    )
  )
}

function pilcrowMark(): JSXNode {
  return createElement('div', {
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
  return createElement('div', {
    style: {
      display  : 'flex',
      left     : Math.round((CARD_WIDTH - TITLE_WIDTH) / 2),
      position : 'absolute',
      top      : TITLE_TOP
    },
    children: createElement('img', { height, src, width: TITLE_WIDTH })
  })
}
