<script setup lang="ts">
import PrimitivesComposition from './PrimitivesComposition.vue'

defineProps<{ primitive: string }>()
</script>

<template>
  <PrimitivesComposition :initial-focus="primitive" />
  <slot name="lead" />
  <slot />
  <h2 id="related" tabindex="-1">
    Related
    <a class="header-anchor" href="#related" aria-label="Permalink to &quot;Related&quot;">&ZeroWidthSpace;</a>
  </h2>
  <slot name="related" />
</template>
