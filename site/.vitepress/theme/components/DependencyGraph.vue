<script setup lang="ts">
import { PRIMITIVES, type PrimitiveSlug } from '../../lib/primitives'
import { useCurrentPrimitive }            from '../../lib/route'

interface Node {
  cx    : number
  cy    : number
  slug  : PrimitiveSlug
  width : number
}

interface Edge {
  d: string
}

const currentSlug = useCurrentPrimitive()

const nodes: Node[] = [
  { slug: 'source',           cx: 100, cy: 50,  width: 70  },
  { slug: 'pipeline',         cx: 380, cy: 50,  width: 80  },
  { slug: 'suppression-map',  cx: 100, cy: 170, width: 120 },
  { slug: 'binding-analysis', cx: 280, cy: 170, width: 130 },
  { slug: 'rule-id',          cx: 460, cy: 170, width: 70  }
]

const edges: Edge[] = [
  { d: 'M170 50 L348 50' },
  { d: 'M100 75 L100 145' },
  { d: 'M120 80 Q220 130, 270 145' },
  { d: 'M390 80 L460 145' }
]
</script>

<template>
  <div class="dep-graph">
    <svg viewBox="0 0 540 220" xmlns="http://www.w3.org/2000/svg" role="img" aria-label="Primitive dependency graph">
      <defs>
        <marker id="arrow" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="6" markerHeight="6" orient="auto-start-reverse">
          <path d="M0 0 L10 5 L0 10 Z" fill="var(--vp-c-divider)" />
        </marker>
      </defs>
      <path v-for="(edge, idx) in edges" :key="idx" class="dep-graph-edge" :d="edge.d" />
      <g v-for="node in nodes" :key="node.slug">
        <rect
          :class="['dep-graph-node', { active: node.slug === currentSlug }]"
          :x="node.cx - node.width / 2"
          :y="node.cy - 18"
          :width="node.width"
          height="36"
          rx="6"
        />
        <text
          class="dep-graph-label"
          :x="node.cx"
          :y="node.cy + 4"
          text-anchor="middle"
        >{{ PRIMITIVES[node.slug] }}</text>
      </g>
    </svg>
  </div>
</template>
