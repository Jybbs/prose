import { PRIMITIVES, type PrimitiveSlug } from './primitives'

export interface DepGraphNode {
  cx    : number
  cy    : number
  slug  : PrimitiveSlug
  width : number
}

export interface DepGraphEdge {
  d: string
}

export const DEP_GRAPH_NODES: readonly DepGraphNode[] = [
  { cx: 280, cy: 170, slug: 'binding-analysis', width: 130 },
  { cx: 380, cy: 50,  slug: 'pipeline',         width: 80  },
  { cx: 460, cy: 170, slug: 'rule-id',          width: 70  },
  { cx: 100, cy: 50,  slug: 'source',           width: 70  },
  { cx: 100, cy: 170, slug: 'suppression-map',  width: 120 }
]

export const DEP_GRAPH_EDGES: readonly DepGraphEdge[] = [
  { d: 'M170 50 L348 50' },
  { d: 'M100 75 L100 145' },
  { d: 'M120 80 Q220 130, 270 145' },
  { d: 'M390 80 L460 145' }
]

const NODE_SLUGS    = new Set(DEP_GRAPH_NODES.map(n => n.slug))
const MISSING_NODES = Object.keys(PRIMITIVES).filter(slug => !NODE_SLUGS.has(slug as PrimitiveSlug))
const EXTRA_NODES   = [...NODE_SLUGS].filter(slug => !(slug in PRIMITIVES))

if (MISSING_NODES.length > 0 || EXTRA_NODES.length > 0) {
  throw new Error(
    `dep-graph nodes out of sync with PRIMITIVES. ` +
    `missing: [${MISSING_NODES.join(', ')}], extra: [${EXTRA_NODES.join(', ')}]`
  )
}
