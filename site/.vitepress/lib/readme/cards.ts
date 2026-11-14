import fs   from 'node:fs'
import path from 'node:path'

import type { JSXNode } from 'satori/jsx'

import { loadBrandAssets }              from '../og/render/assets'
import { el, FONT, toSvg }              from '../og/render/parts'
import { discoverRuleIndex }            from '../rules/discovery'
import * as palette                     from '../shared/palette'
import { readmeDir, rulesDir, siteDir } from '../shared/paths'

interface Card {
  hue      : keyof typeof palette.HUES
  label    : string
  page     : string
  specimen : Specimen
}

type Specimen =
  | { kind: 'code', lines: readonly string[] }
  | { kind: 'prose', lines: readonly string[] }
  | { kind: 'rules', slugs: readonly string[] }

// Holds the cards in the row's order, each a specimen of the page it opens.
export const CARDS: readonly Card[] = [
  {
    hue      : 'ube',
    label    : 'Read',
    page     : 'the guide',
    specimen : {
      kind  : 'prose',
      lines : ['Code is read far more often', 'than it is written, and the eye', 'looks for parallels and shape.']
    }
  },
  {
    hue      : 'celadon',
    label    : 'Browse',
    page     : 'the rules',
    specimen : { kind: 'rules', slugs: ['align-equals', 'reflow-calls', 'alphabetize-siblings'] }
  },
  {
    hue      : 'whiskey',
    label    : 'Try',
    page     : 'the sandbox',
    specimen : { kind: 'code', lines: ['timeout = 30', 'retries = 5', 'backoff = 1.8'] }
  }
]

// Holds each card's size, the band's height, the contrast ratios the cursor and
// the label take, and the share of the hue the band and the border take over
// the page ground in each scheme.
const CARD_STYLE = {
  bandHeight : 94,
  border     : { dark: 34, light: 55 },
  height     : 168,
  ratios     : { cursor: 3, label: 7 },
  shade      : { dark: 26, light: 38 },
  width      : 260
} as const

// Draws every card in each scheme with the brand fonts, keyed
// `<label>-<scheme>` in lowercase.
export async function drawCards(): Promise<Record<string, string>> {
  const { fonts } = loadBrandAssets(siteDir(import.meta.url))
  const rules     = discoverRuleIndex(rulesDir(import.meta.url))
  const drawn     = CARDS.flatMap(card => palette.SCHEMES.map(async scheme => [
    `${card.label.toLowerCase()}-${scheme}`,
    await toSvg(cardNode(card, scheme, slug => rules.get(slug)!.family), fonts, CARD_STYLE.height, CARD_STYLE.width)
  ] as const))
  return Object.fromEntries(await Promise.all(drawn))
}

function cardNode(
  card     : Card,
  scheme   : palette.Scheme,
  familyOf : (slug: string) => keyof typeof palette.FAMILIES
): JSXNode {
  const hue    = palette.HUES[card.hue]
  const ground = palette.GROUNDS.page[scheme]
  const band   = palette.srgbMix(ground, hue, CARD_STYLE.shade[scheme])
  const ink    = palette.VITEPRESS['text-1'][scheme]
  const label  = palette.shadeReading(ground, hue, CARD_STYLE.ratios.label)
  return el('div',
    {
      style: {
        backgroundColor : ground,
        border          : `1px solid ${palette.srgbMix(ground, hue, CARD_STYLE.border[scheme])}`,
        borderRadius    : 12,
        boxSizing       : 'border-box',
        display         : 'flex',
        flexDirection   : 'column',
        height          : '100%',
        overflow        : 'hidden',
        width           : '100%'
      }
    },
    el('div',
      {
        style: {
          backgroundColor : band,
          display         : 'flex',
          flexDirection   : 'column',
          height          : CARD_STYLE.bandHeight,
          justifyContent  : 'center',
          padding         : '0 20px'
        }
      },
      ...specimenLines(card.specimen, { band, hue, ink }, familyOf)
    ),
    el('div',
      {
        style: {
          alignItems     : 'center',
          display        : 'flex',
          flexGrow       : 1,
          justifyContent : 'space-between',
          padding        : '0 20px'
        }
      },
      el('div', { children: card.label, style: { color: label, fontFamily: FONT.body, fontSize: 31 } }),
      el('div',
        { style: { alignItems: 'center', color: ink, display: 'flex', fontFamily: FONT.body, fontSize: 13, fontStyle: 'italic', gap: 5 } },
        card.page,
        arrow(ink)
      )
    )
  )
}

// Draws a right arrow as a path, since the Lora subset the cards load holds
// no glyph for one.
function arrow(color: string): JSXNode {
  return el('svg', { height: 9, viewBox: '0 0 12 9', width: 12 },
    el('path', { d: 'M0 4.5h11M7 0.5l4 4-4 4', fill: 'none', stroke: color, strokeWidth: 1.2 }))
}

function specimenLines(
  specimen          : Specimen,
  { band, hue, ink }: { band: string, hue: string, ink: string },
  familyOf          : (slug: string) => keyof typeof palette.FAMILIES
): JSXNode[] {
  const mono = { color: ink, display: 'flex', fontFamily: FONT.mono, fontSize: 12, lineHeight: 1.55 }
  switch (specimen.kind) {
    case 'prose':
      return specimen.lines.map(line =>
        el('div', { children: line, style: { color: ink, fontFamily: FONT.body, fontSize: 13, fontStyle: 'italic', lineHeight: 1.4 } }))
    case 'rules':
      return specimen.slugs.map(slug => el('div', { style: { ...mono, alignItems: 'center', gap: 8 } },
        el('div', {
          style: {
            backgroundColor : palette.shadeReading(band, palette.FAMILIES[familyOf(slug)], palette.ACCENT_RATIOS.edge),
            borderRadius    : 4,
            height          : 7,
            width           : 7
          }
        }),
        slug
      ))
    case 'code':
      return specimen.lines.map((line, index) => el('div', { style: mono },
        line,
        ...(index === specimen.lines.length - 1
          ? [el('div', { style: { backgroundColor: palette.shadeReading(band, hue, CARD_STYLE.ratios.cursor), height: 15, width: 7 } })]
          : [])
      ))
  }
}

if (import.meta.main) {
  const outDir = readmeDir(import.meta.url)
  fs.mkdirSync(outDir, { recursive: true })
  for (const [name, svg] of Object.entries(await drawCards())) fs.writeFileSync(path.join(outDir, `${name}.svg`), svg)
}
