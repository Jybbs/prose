<script setup lang="ts">
import { computed } from 'vue'

import { DEP_GRAPH_EDGES, DEP_GRAPH_NODES } from './dependency-graph-data'
import { PRIMITIVES }                       from '../../../lib/shared/registries'
import { useCurrentPrimitive }              from '../../../lib/composables/route'

const current     = useCurrentPrimitive()
const currentSlug = computed(() => current.value?.slug ?? null)
</script>

<template>
  <div class="dep-graph">
    <svg viewBox="0 0 540 220" xmlns="http://www.w3.org/2000/svg" role="img" aria-label="Primitive dependency graph">
      <defs>
        <marker id="arrow" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="6" markerHeight="6" orient="auto-start-reverse">
          <path d="M0 0 L10 5 L0 10 Z" fill="var(--vp-c-divider)" />
        </marker>
      </defs>
      <path v-for="(edge, idx) in DEP_GRAPH_EDGES" :key="idx" class="dep-graph-edge" :d="edge.d" />
      <g v-for="node in DEP_GRAPH_NODES" :key="node.slug">
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
