import { Resvg } from '@resvg/resvg-js'
import satori    from 'satori'

import type { Font } from 'satori'

import type { JsxNode } from './template'

const CARD_HEIGHT = 630
const CARD_WIDTH  = 1200

export async function renderCard(node: JsxNode, fonts: Font[]): Promise<Buffer> {
  const svg = await satori(node, { fonts, height: CARD_HEIGHT, width: CARD_WIDTH })
  return new Resvg(svg).render().asPng()
}
