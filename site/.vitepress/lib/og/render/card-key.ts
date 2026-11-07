import { hash } from 'ohash'

import type { RenderGraph } from '../../shared/render-graph'
import type { BrandAssets } from './assets'
import type { Card }        from './pool'

export function cardKeyer(
  brand   : BrandAssets,
  graph   : RenderGraph,
  version : string
): (card: Card) => string {
  const base = { brand: hash(brand), graph: hash(graph), version }
  return card => hash({ base, card })
}
