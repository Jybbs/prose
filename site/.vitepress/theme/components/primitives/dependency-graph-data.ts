import type { PrimitiveSlug } from '../../../lib/shared/registries'

export interface DepGraphNode {
  cx    : number
  cy    : number
  slug  : PrimitiveSlug
  width : number
}

export interface DepGraphEdge {
  d: string
}

export const DEP_GRAPH_NODES = [
  { cx: 280, cy: 170, slug: 'binding-analysis', width: 130 },
  { cx: 380, cy: 50,  slug: 'pipeline',         width: 80  },
  { cx: 460, cy: 170, slug: 'rule-id',          width: 70  },
  { cx: 100, cy: 50,  slug: 'source',           width: 70  },
  { cx: 100, cy: 170, slug: 'suppression-map',  width: 120 }
] as const satisfies readonly DepGraphNode[]

export const DEP_GRAPH_EDGES: readonly DepGraphEdge[] = [
  { d: 'M170 50 L348 50' },
  { d: 'M100 75 L100 145' },
  { d: 'M120 80 Q220 130, 270 145' },
  { d: 'M390 80 L460 145' }
]
